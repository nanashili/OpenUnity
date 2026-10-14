//! Native shell for the independent authoring contract INFRA-EDITOR-001.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use eframe::egui::{self, Context};
use openunity_editor_core::capabilities::{self, Capability};
use openunity_editor_core::console::{Console, Severity};
use openunity_editor_core::document::Document;
use openunity_editor_core::imported_scene::ImportedDocument;
use openunity_editor_core::imported_scene_io;
use openunity_editor_core::preferences::Preferences;
use openunity_editor_core::project::Project;
use openunity_editor_core::scene::Scene;
use openunity_editor_core::scene_io;
use openunity_editor_core::shortcuts::{self, Chord, Command, Key, Modifiers};
use openunity_editor_core::write_guard::WritePolicy;

use crate::EditorAction;
use crate::appearance::{self, AppearanceProfile};
use crate::component_expansion::{ComponentExpansionController, ComponentExpansionSnapshot};
use crate::inspector_lock::{InspectorController, InspectorSnapshot, InspectorTarget};
use crate::panels::hierarchy_browser::HierarchyBrowser;
use crate::panels::{
    console::ConsolePanel, hierarchy, imported, inspector::InspectorPanel,
    project::ProjectBrowserPanel, viewport::ViewportPanel,
};
use crate::workspace::{TabId, Workspace, WorkspaceSnapshot};

/// Explicit startup inputs; no default preference or project file is read.
#[derive(Default)]
pub struct AppOptions {
    /// Optional source project to discover without modification.
    pub project_path: Option<PathBuf>,
    /// Optional independent scene to load.
    pub scene_path: Option<PathBuf>,
    /// Native smoke receipt shared with the process entry point.
    pub smoke: Option<Arc<Mutex<SmokeStatus>>>,
}

/// Receipt from actual native screenshot delivery, distinct from headless UI checks.
#[derive(Default)]
pub struct SmokeStatus {
    /// Actual shell profile observed during native updates.
    pub appearance_profile: Option<String>,
    /// Actual last native workspace allocation and controls.
    pub workspace_snapshot: Option<serde_json::Value>,
    /// Actual last native appearance and active font definitions.
    pub appearance_snapshot: Option<serde_json::Value>,
    /// Number of native app update calls observed.
    pub frames: u64,
    /// Actual GPU readback dimensions, when delivered by the native backend.
    pub screenshot_size: Option<[usize; 2]>,
    /// Actual screenshot pixels, retained for an optional verification artifact.
    pub screenshot_image: Option<egui::ColorImage>,
    /// True only after a nonempty native screenshot and at least three updates.
    pub completed: bool,
    /// Explicit timeout/failure diagnostic.
    pub error: Option<String>,
    /// Complete active independent document snapshot captured by the native frame.
    pub document_snapshot: Option<serde_json::Value>,
}

#[derive(Clone, Copy)]
enum PathFormKind {
    OpenProject,
    OpenScene,
    Import,
    SaveAs,
}
struct PathForm {
    kind: PathFormKind,
    primary: String,
    bindings: String,
    error: Option<String>,
}

enum Pending {
    New,
    Open(Scene, PathBuf),
    Imported(Box<ImportedDocument>, Option<PathBuf>),
    Close,
}

/// User decision for replacing or closing an unsaved document.
#[derive(Clone, Copy)]
pub enum UnsavedChoice {
    /// Persist successfully before continuing; failure keeps the guard open.
    Save,
    /// Continue without persisting this document.
    Discard,
    /// Keep the current document and cancel the pending action.
    Cancel,
}

/// View-only requests emitted by the actual Reference Inspector header.
pub enum InspectorHeaderAction {
    /// Resolve the current lock boolean and availability at ordered dispatch.
    ToggleLock,
    /// Resolve current membership for the menu's captured target identity.
    SetAllComponents {
        /// Displayed identity captured when the actual pane menu opened.
        context: InspectorTarget,
        /// Requested presentation state for current live membership.
        expanded: bool,
    },
}

/// The native app and its document-scoped state.
pub struct EditorApp {
    document: Document,
    imported: Option<ImportedDocument>,
    provenance_roots: Vec<PathBuf>,
    imported_inspector: imported::InspectorPanel,
    imported_viewport: imported::ViewportPanel,
    bindings_text: String,
    project: Option<Project>,
    save_path: Option<PathBuf>,
    protected_roots: Vec<PathBuf>,
    forbidden_reads: Vec<PathBuf>,
    policy: WritePolicy,
    workspace: Workspace,
    hierarchy_browser: HierarchyBrowser,
    hierarchy_expansion: hierarchy::ExpansionState,
    inspector_scroll: serde_json::Value,
    inspector_scope: u64,
    inspector_controller: InspectorController,
    inspector_controls: Vec<serde_json::Value>,
    inspector_menu_popup: Option<egui::Id>,
    inspector_component_menu_context: Option<InspectorTarget>,
    component_expansion: ComponentExpansionController,
    resource_snapshot: serde_json::Value,
    path_form: Option<PathForm>,
    preferences: Preferences,
    console: Console,
    project_panel: ProjectBrowserPanel,
    inspector_panel: InspectorPanel,
    console_panel: ConsolePanel,
    viewport_panel: ViewportPanel,
    project_text: String,
    scene_text: String,
    status: String,
    pending: Option<Pending>,
    close_allowed: bool,
    smoke: Option<Arc<Mutex<SmokeStatus>>>,
    smoke_started: Instant,
    smoke_requested: bool,
    frames: u64,
}

mod actions;
mod commands;
mod component_expansion;
mod console;
mod frame;
mod history;
mod inspector_actions;
mod inspector_header;
mod inspector_menu;
mod inspector_state;
mod loading;
mod menus;
mod observation;
mod path_forms;
mod paths;
mod project_warnings;
mod rendering;
mod replacement;
mod saving;
mod startup;
mod toolbar;
mod transport;

#[cfg(test)]
mod tests;

impl eframe::App for EditorApp {
    fn update(&mut self, ctx: &Context, _frame: &mut eframe::Frame) {
        self.show(ctx);
    }
}
