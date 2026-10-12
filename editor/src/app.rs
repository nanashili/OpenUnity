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
impl PathFormKind {
    fn title(self) -> &'static str {
        match self {
            Self::OpenProject => "Open Project",
            Self::OpenScene => "Open Scene/Archive",
            Self::Import => "Import Saved Scene",
            Self::SaveAs => "Save As",
        }
    }
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

impl EditorApp {
    /// Construct a validated app; startup filesystem/model errors are returned.
    pub fn new(options: AppOptions) -> Result<Self, String> {
        let mut candidates = vec![PathBuf::from("/Applications")];
        if let Some(home) = std::env::var_os("HOME") {
            let home = PathBuf::from(home);
            candidates.push(home.join(".local/share/openunity-research"));
            candidates.push(home.join("Library/Application Support/OpenUnityPrivateResearch"));
        }
        let mut protected_roots = Vec::new();
        // Directory metadata only: never inspect reference/research contents.
        for path in candidates {
            match fs::symlink_metadata(&path) {
                Ok(_) => protected_roots.push(path),
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(error) => return Err(format!("protect {}: {error}", path.display())),
            }
        }
        let policy = WritePolicy::new(protected_roots.clone()).map_err(|e| e.to_string())?;
        let preferences = Preferences::default();
        let mut app = Self {
            document: Document::new(
                Scene::new("Untitled").map_err(|e| e.to_string())?,
                preferences.history_capacity,
            ),
            imported: None,
            provenance_roots: Vec::new(),
            imported_inspector: imported::InspectorPanel::default(),
            imported_viewport: imported::ViewportPanel::default(),
            bindings_text: String::new(),
            project: None,
            save_path: None,
            forbidden_reads: protected_roots.clone(),
            protected_roots,
            policy,
            workspace: Workspace::default(),
            hierarchy_browser: HierarchyBrowser::default(),
            inspector_scroll: serde_json::Value::Null,
            inspector_scope: 1,
            inspector_controller: InspectorController::deferred(),
            inspector_controls: Vec::new(),
            inspector_menu_popup: None,
            inspector_component_menu_context: None,
            component_expansion: ComponentExpansionController::default(),
            resource_snapshot: serde_json::Value::Null,
            path_form: None,
            preferences,
            console: Console::new(512).map_err(|e| e.to_string())?,
            project_panel: ProjectBrowserPanel::default(),
            inspector_panel: InspectorPanel::default(),
            console_panel: ConsolePanel::default(),
            viewport_panel: ViewportPanel::default(),
            project_text: String::new(),
            scene_text: String::new(),
            status: "Independent OpenUnity authoring; Unity compatibility is unverified.".into(),
            pending: None,
            close_allowed: false,
            smoke: options.smoke,
            smoke_started: Instant::now(),
            smoke_requested: false,
            frames: 0,
        };
        if let Some(path) = options.project_path {
            app.open_project(&path)?;
        }
        if let Some(path) = options.scene_path {
            let candidate = app.load_document(&path)?;
            app.protect_candidate(&candidate);
            app.perform(candidate)?;
        }
        Ok(app)
    }

    /// Legacy authoring-only view. Use imported_document for the active imported mode.
    /// This inactive legacy snapshot is never a save or mutation source in imported mode.
    pub fn document(&self) -> &Document {
        &self.document
    }

    /// Active typed imported document, or None in independent primitive authoring mode.
    pub fn imported_document(&self) -> Option<&ImportedDocument> {
        self.imported.as_ref()
    }

    /// Actual rendered workspace geometry and accessible control identities.
    pub fn workspace_snapshot(&self) -> &WorkspaceSnapshot {
        self.workspace.snapshot()
    }
    /// Replace only one tab's independent session title metadata, atomically.
    pub fn replace_tab_title(
        &mut self,
        tab: TabId,
        metadata: crate::tab_titles::TabTitleMetadata,
    ) -> Result<(), String> {
        self.workspace.replace_tab_title(tab, metadata)
    }
    /// Restore one tab's fixed clean title, without changing document or Inspector state.
    pub fn clear_tab_title(&mut self, tab: TabId) {
        self.workspace.clear_tab_title(tab);
    }
    /// Read one complete title input record retained for this UI session.
    pub fn tab_title_metadata(&self, tab: TabId) -> &crate::tab_titles::TabTitleMetadata {
        self.workspace.tab_title_metadata(tab)
    }
    /// Select a reviewed shell profile without replacing documents or Inspector drafts.
    pub fn set_appearance_profile(&mut self, profile: AppearanceProfile) {
        self.workspace.set_profile(profile);
        self.reconcile_inspector();
    }

    /// Actual Console view state, existing retained records and current core counters.
    pub fn console_snapshot(&self) -> serde_json::Value {
        let mut value = self.console_panel.snapshot();
        value["profile"] = self.workspace.profile().name().into();
        value["capacity"] = self.console.capacity().into();
        value["dropped_count"] = self.console.dropped_count().into();
        value["records"] = self.console.records().map(|record|serde_json::json!({"sequence":record.sequence,"severity":crate::console_detail::severity_label(record.severity),"message":record.message})).collect::<Vec<_>>().into();
        value
    }
    /// Capture an existing record sequence with the actual current Console owner.
    pub fn console_identity(
        &self,
        sequence: u64,
    ) -> Option<crate::console_detail::ConsoleRecordIdentity> {
        self.console_panel.identity(sequence)
    }
    /// Select a retained currently visible Console record without touching editor documents.
    pub fn select_console_record(
        &mut self,
        identity: crate::console_detail::ConsoleRecordIdentity,
    ) -> Result<(), String> {
        self.check_idle()?;
        if self.workspace.profile() != AppearanceProfile::Reference {
            return Err("Console detail selection requires Reference appearance".into());
        }
        let result = self.console_panel.select(identity, &self.console);
        if let Err(error) = &result {
            self.status.clone_from(error);
        }
        result
    }
    /// Replace only the existing Console literal message filter.
    pub fn set_console_filter(&mut self, filter: String) {
        self.console_panel.set_filter(filter, &self.console);
    }
    /// Replace only the existing Console severity filter values.
    pub fn set_console_severities(&mut self, severities: &[Severity]) {
        self.console_panel.set_severities(severities, &self.console);
    }
    /// Append an independent editor record through the unchanged bounded Console API.
    pub fn append_console_record(
        &mut self,
        severity: Severity,
        message: &str,
    ) -> Result<u64, String> {
        let sequence = self
            .console
            .push(severity, message)
            .map_err(|error| error.to_string())?;
        if self.workspace.profile() == AppearanceProfile::Reference {
            self.console_panel.reconcile(&self.console);
        }
        Ok(sequence)
    }
    /// Clear retained Console records using existing counter-preserving semantics.
    pub fn clear_console_records(&mut self) {
        self.console.clear();
        if self.workspace.profile() == AppearanceProfile::Reference {
            self.console_panel.reconcile(&self.console);
        }
    }
    /// Explicitly bind a new empty Console lifetime before any new record selection events.
    /// Capacity validation and lifetime allocation both precede replacement; failure is atomic.
    pub fn replace_console_owner(&mut self, capacity: usize) -> Result<(), String> {
        let console = Console::new(capacity).map_err(|error| error.to_string())?;
        self.console_panel.reset_owner()?;
        self.console = console;
        Ok(())
    }
    /// Actual committed document-scoped Inspector view and last rendered controls.
    pub fn inspector_snapshot(&self) -> serde_json::Value {
        let mut value = self.inspector_controller.snapshot().to_json();
        value["profile"] = self.workspace.profile().name().into();
        value["controls"] = self.inspector_controls.clone().into();
        value["component_expansion"] = self.component_expansion.snapshot().to_json();
        value["component_menu_context"] = self.inspector_component_menu_context.map(|t|serde_json::json!({"document_scope":t.document_scope,"entity_id":t.entity_id.0})).into();
        value["draft"] = if self.imported.is_some() {
            self.imported_inspector.draft_snapshot()
        } else {
            self.inspector_panel.draft_snapshot()
        };
        value["panel_controls"] = if self.imported.is_some() {
            self.imported_inspector.controls()
        } else {
            self.inspector_panel.controls()
        }
        .to_vec()
        .into();
        value
    }
    /// Current explicitly addressed Inspector target.
    pub fn inspector_target(&self) -> Option<InspectorTarget> {
        self.inspector_controller.snapshot().target
    }
    /// Observe changed lock state after its target and draft state commit.
    pub fn observe_inspector_lock(&mut self, observer: impl FnMut(&InspectorSnapshot) + 'static) {
        self.inspector_controller.observe(observer);
    }
    /// Observe complete changed component presentation results after commit.
    pub fn observe_component_expansion(
        &mut self,
        observer: impl FnMut(&ComponentExpansionSnapshot) + 'static,
    ) {
        self.component_expansion.observe(observer);
    }
    fn reconcile_component_expansion(&mut self) {
        let mut live = Vec::new();
        if let Some(document) = &self.imported {
            let supported: std::collections::BTreeSet<_> = document
                .plan()
                .transforms
                .iter()
                .map(|c| c.file_id)
                .chain(document.plan().scripts.iter().map(|c| c.file_id))
                .collect();
            let objects: std::collections::BTreeMap<_, _> = document
                .plan()
                .game_objects
                .iter()
                .map(|g| (g.file_id, g))
                .collect();
            for mapping in document.snapshot().editor_ids() {
                if let Some(object) = objects.get(&mapping.game_object_file_id) {
                    live.push((
                        InspectorTarget {
                            document_scope: self.inspector_scope,
                            entity_id: mapping.entity_id,
                        },
                        object
                            .components
                            .iter()
                            .filter(|id| supported.contains(id))
                            .map(|id| (*id, true))
                            .collect(),
                    ));
                }
            }
        }
        self.component_expansion
            .reconcile(self.inspector_scope, self.inspector_target(), &live);
    }
    /// Apply an explicitly scoped individual component presentation request.
    pub fn set_inspector_component_expanded(
        &mut self,
        context: InspectorTarget,
        file_id: i64,
        expanded: bool,
    ) -> Result<(), String> {
        self.check_idle()?;
        if self.workspace.profile() != AppearanceProfile::Reference {
            return Err("Component expansion is unavailable in Legacy appearance.".into());
        }
        self.reconcile_inspector();
        self.component_expansion
            .set_component(context, file_id, expanded)
    }
    /// Apply a bulk request to current live membership of a captured displayed target.
    pub fn set_all_inspector_components_expanded(
        &mut self,
        context: InspectorTarget,
        expanded: bool,
    ) -> Result<(), String> {
        self.check_idle()?;
        if self.workspace.profile() != AppearanceProfile::Reference {
            return Err("Component expansion is unavailable in Legacy appearance.".into());
        }
        self.reconcile_inspector();
        self.component_expansion.set_all(context, expanded)
    }
    fn inspector_selection(&self) -> Option<InspectorTarget> {
        let id = self.imported.as_ref().map_or_else(
            || self.document.selection().primary(),
            ImportedDocument::primary,
        );
        id.map(|entity_id| InspectorTarget {
            document_scope: self.inspector_scope,
            entity_id,
        })
    }
    fn reconcile_inspector(&mut self) {
        if self.workspace.profile() != AppearanceProfile::Reference {
            return;
        }
        let before = self.inspector_target();
        let selection = self.inspector_selection();
        let imported = &self.imported;
        let document = &self.document;
        self.inspector_controller
            .reconcile(self.inspector_scope, selection, |t| {
                imported.as_ref().map_or_else(
                    || document.scene().get(t.entity_id).is_some(),
                    |d| d.entity(t.entity_id).is_ok(),
                )
            });
        if before != self.inspector_target() {
            self.inspector_panel = InspectorPanel::default();
            self.imported_inspector = imported::InspectorPanel::default();
        }
        self.reconcile_component_expansion();
        self.inspector_controller.deliver_notifications();
    }
    /// Set Reference Inspector view state without mutating document or selection.
    pub fn set_inspector_lock(&mut self, locked: bool) -> Result<(), String> {
        self.check_idle()?;
        if self.workspace.profile() != AppearanceProfile::Reference {
            return Err("Inspector locking is unavailable in Legacy appearance.".into());
        }
        let before = self.inspector_target();
        let selection = self.inspector_selection();
        let imported = &self.imported;
        let document = &self.document;
        self.inspector_controller
            .set_lock(locked, self.inspector_scope, selection, |t| {
                imported.as_ref().map_or_else(
                    || document.scene().get(t.entity_id).is_some(),
                    |d| d.entity(t.entity_id).is_ok(),
                )
            })?;
        if before != self.inspector_target() {
            self.inspector_panel = InspectorPanel::default();
            self.imported_inspector = imported::InspectorPanel::default();
        }
        self.inspector_controller.deliver_notifications();
        self.reconcile_component_expansion();
        Ok(())
    }
    /// Toggle using current committed state, including activations from an open menu.
    pub fn toggle_inspector_lock(&mut self) -> Result<(), String> {
        self.set_inspector_lock(!self.inspector_controller.snapshot().locked)
    }
    fn validate_inspector_target(&self, target: InspectorTarget) -> Result<(), String> {
        self.inspector_controller
            .validate_target(target, self.inspector_scope, |t| {
                self.imported.as_ref().map_or_else(
                    || self.document.scene().get(t.entity_id).is_some(),
                    |d| d.entity(t.entity_id).is_ok(),
                )
            })?;
        if self.inspector_target() != Some(target) {
            return Err("Inspector edit rejected: target is no longer displayed.".into());
        }
        if self
            .inspector_controller
            .snapshot()
            .draft_target
            .is_some_and(|owner| owner != target)
        {
            return Err("Inspector edit rejected: draft belongs to another target.".into());
        }
        Ok(())
    }
    /// Execute only a supported authoring edit addressed to its captured document scope.
    pub fn apply_inspector_action(
        &mut self,
        target: InspectorTarget,
        action: EditorAction,
    ) -> Result<(), String> {
        self.check_idle()?;
        self.validate_inspector_target(target)?;
        if !matches!(&action, EditorAction::Edit { id, .. } if *id == target.entity_id) {
            return Err("Inspector edit does not address its captured target.".into());
        }
        self.apply_action(action)?;
        self.inspector_panel = InspectorPanel::default();
        self.inspector_controller.draft(None, false);
        Ok(())
    }
    /// Execute an imported property/component edit with scope, liveness and owner validation.
    pub fn apply_imported_inspector_action(
        &mut self,
        target: InspectorTarget,
        action: imported::ImportedAction,
    ) -> Result<(), String> {
        self.check_idle()?;
        self.validate_inspector_target(target)?;
        let document = self
            .imported
            .as_ref()
            .ok_or("No imported document is active.")?;
        let property_edit = matches!(&action, imported::ImportedAction::EditEntity { id, .. } if *id == target.entity_id);
        let owned_script = match &action {
            imported::ImportedAction::SetScriptEnabled { script_file_id, .. } => document.components(target.entity_id).map_err(|e| e.to_string())?.iter().any(|c| matches!(c, openunity_editor_core::imported_scene::ImportedComponent::Script(s) if s.file_id == *script_file_id)),
            _ => false,
        };
        if !property_edit && !owned_script {
            return Err(
                "Imported Inspector edit does not address its captured target/component.".into(),
            );
        }
        self.apply_imported_action(action)?;
        if property_edit {
            self.imported_inspector = imported::InspectorPanel::default();
            self.inspector_controller.draft(None, false);
        }
        Ok(())
    }
    /// Actual last-frame Project browser state, paths, and rendered controls.
    pub fn project_browser_snapshot(&self) -> serde_json::Value {
        self.project_panel.snapshot()
    }
    /// Actual last-frame hierarchy query, visible canonical identities, and controls.
    pub fn hierarchy_browser_snapshot(&self) -> serde_json::Value {
        self.hierarchy_browser.snapshot()
    }
    /// Actual shell appearance and Inspector scroll allocations from the last frame.
    pub fn appearance_snapshot(&self) -> serde_json::Value {
        serde_json::json!({"profile":self.workspace.profile().name(),"inspector_scroll":self.inspector_scroll,"resources":self.resource_snapshot})
    }

    /// Whether the active document differs from its last successful save.
    pub fn is_dirty(&self) -> bool {
        self.imported
            .as_ref()
            .map_or_else(|| self.document.is_dirty(), ImportedDocument::is_dirty)
    }

    /// Full active semantic archive, without transient selection or history.
    pub fn semantic_snapshot(&self) -> Result<serde_json::Value, String> {
        let bytes = if let Some(document) = &self.imported {
            imported_scene_io::encode(document.snapshot()).map_err(|e| e.to_string())?
        } else {
            scene_io::encode(self.document.scene()).map_err(|e| e.to_string())?
        };
        serde_json::from_slice(&bytes).map_err(|e| e.to_string())
    }

    fn check_idle(&self) -> Result<(), String> {
        if self.pending.is_some() {
            Err("Resolve the pending unsaved-work decision first.".into())
        } else {
            Ok(())
        }
    }

    /// Apply the actual active document undo operation.
    pub fn undo(&mut self) -> Result<bool, String> {
        self.check_idle()?;
        let changed = if let Some(document) = &mut self.imported {
            document.undo().map_err(|e| e.to_string())?
        } else {
            self.document.undo().map_err(|e| e.to_string())?
        };
        if changed {
            if self.workspace.profile() == AppearanceProfile::Legacy {
                self.imported_inspector = imported::InspectorPanel::default();
            }
            self.reconcile_inspector();
        }
        Ok(changed)
    }

    /// Apply the actual active document redo operation.
    pub fn redo(&mut self) -> Result<bool, String> {
        self.check_idle()?;
        let changed = if let Some(document) = &mut self.imported {
            document.redo().map_err(|e| e.to_string())?
        } else {
            self.document.redo().map_err(|e| e.to_string())?
        };
        if changed {
            if self.workspace.profile() == AppearanceProfile::Legacy {
                self.imported_inspector = imported::InspectorPanel::default();
            }
            self.reconcile_inspector();
        }
        Ok(changed)
    }

    fn can_undo(&self) -> bool {
        self.imported
            .as_ref()
            .map_or_else(|| self.document.can_undo(), ImportedDocument::can_undo)
    }

    fn can_redo(&self) -> bool {
        self.imported
            .as_ref()
            .map_or_else(|| self.document.can_redo(), ImportedDocument::can_redo)
    }

    /// Validate an output path against current and archived source provenance.
    /// Missing archived source roots still exclude their future descendants.
    pub fn check_output_path(&self, path: &Path) -> Result<(), String> {
        self.policy.check_read(path).map_err(|e| e.to_string())?;
        let absolute = if path.is_absolute() {
            path.to_path_buf()
        } else {
            std::env::current_dir()
                .map_err(|e| e.to_string())?
                .join(path)
        };
        let normalized: PathBuf = absolute
            .components()
            .filter(|c| !matches!(c, std::path::Component::CurDir))
            .collect();
        if self
            .provenance_roots
            .iter()
            .chain(&self.protected_roots)
            .any(|root| normalized.starts_with(root))
        {
            return Err("Writes within a protected source or reference root are forbidden.".into());
        }
        Ok(())
    }

    fn protect_candidate(&mut self, candidate: &Pending) {
        if let Pending::Imported(document, _) = candidate {
            let root = &document.source().project_root;
            if !self.provenance_roots.contains(root) {
                self.provenance_roots.push(root.clone());
            }
        }
    }

    fn load_document(&self, path: &Path) -> Result<Pending, String> {
        if path
            .extension()
            .is_some_and(|extension| extension == "ouimport")
        {
            self.check_input(path)?;
            let snapshot =
                imported_scene_io::load(path, &self.policy).map_err(|e| e.to_string())?;
            let mut document =
                ImportedDocument::from_snapshot(snapshot, self.preferences.history_capacity)
                    .map_err(|e| e.to_string())?;
            if self
                .forbidden_reads
                .iter()
                .any(|root| document.source().project_root.starts_with(root))
            {
                return Err("Archive provenance cannot name a reference installation or restricted research root.".into());
            }
            document.mark_saved();
            Ok(Pending::Imported(
                Box::new(document),
                Some(path.to_path_buf()),
            ))
        } else {
            Ok(Pending::Open(self.load_scene(path)?, path.to_path_buf()))
        }
    }

    /// Read a bounded scene from the explicitly opened source project.
    pub fn request_import(&mut self, scene: &Path, bindings: &Path) -> Result<(), String> {
        self.check_idle()?;
        let project = self
            .project
            .as_ref()
            .ok_or("Open a source project before importing a saved scene.")?;
        let scene = if scene.is_absolute() {
            scene.to_path_buf()
        } else {
            project.root().join(scene)
        };
        let bindings = if bindings.is_absolute() {
            bindings.to_path_buf()
        } else {
            project.root().join(bindings)
        };
        self.check_input(&scene)?;
        self.check_input(&bindings)?;
        let document = ImportedDocument::import_project(
            project.root(),
            &scene,
            &bindings,
            self.preferences.history_capacity,
        )
        .map_err(|e| e.to_string())?;
        let candidate = Pending::Imported(Box::new(document), None);
        self.protect_candidate(&candidate);
        self.request(candidate)
    }

    /// Execute an imported inspector/selection transaction on the authoritative plan.
    pub fn apply_imported_action(
        &mut self,
        action: imported::ImportedAction,
    ) -> Result<(), String> {
        self.check_idle()?;
        if matches!(
            action,
            imported::ImportedAction::SetComponentExpanded { .. }
        ) {
            return Err("Component expansion requires a scoped Inspector request.".into());
        }
        let document = self
            .imported
            .as_mut()
            .ok_or("No imported document is active.")?;
        match action {
            imported::ImportedAction::SetComponentExpanded { .. } => {
                unreachable!("checked scoped presentation request")
            }
            imported::ImportedAction::Select { id, toggle } => document.select(id, toggle),
            imported::ImportedAction::EditEntity { id, edit } => {
                document.edit_entity(id, edit).map(|_| ())
            }
            imported::ImportedAction::SetScriptEnabled {
                script_file_id,
                enabled,
            } => document
                .set_script_enabled(script_file_id, enabled)
                .map(|_| ()),
        }
        .map_err(|e| e.to_string())?;
        self.reconcile_inspector();
        Ok(())
    }

    /// Path of the successfully opened or saved document.
    pub fn save_path(&self) -> Option<&Path> {
        self.save_path.as_deref()
    }

    fn check_input(&self, path: &Path) -> Result<(), String> {
        self.policy.check_read(path).map_err(|e| e.to_string())?;
        let absolute =
            fs::canonicalize(path).map_err(|e| format!("open {}: {e}", path.display()))?;
        if self
            .forbidden_reads
            .iter()
            .any(|root| absolute.starts_with(root))
        {
            return Err(
                "Opening reference installation or restricted research inputs is unavailable."
                    .into(),
            );
        }
        Ok(())
    }

    /// Read a project and retain protection for all previously opened source roots.
    pub fn open_project(&mut self, path: &Path) -> Result<(), String> {
        self.check_input(path)?;
        let project = Project::open(path).map_err(|e| e.to_string())?;
        let mut roots = self.protected_roots.clone();
        roots.push(project.root().to_path_buf());
        let policy = WritePolicy::new(roots.clone()).map_err(|e| e.to_string())?;
        for diagnostic in project.diagnostics() {
            let _ = self.console.push(
                Severity::Warning,
                format!(
                    "{}: {}",
                    diagnostic.relative_path.display(),
                    diagnostic.message
                ),
            );
        }
        self.project_text = project.root().display().to_string();
        self.protected_roots = roots;
        self.policy = policy;
        self.project = Some(project);
        Ok(())
    }

    fn load_scene(&self, path: &Path) -> Result<Scene, String> {
        if path
            .extension()
            .is_some_and(|extension| extension == "unity")
        {
            return Err(capabilities::attempt(Capability::UnitySceneImport)
                .unwrap_err()
                .to_string());
        }
        self.check_input(path)?;
        scene_io::load(path, &self.policy).map_err(|e| e.to_string())
    }

    /// Persist to an explicit path. Save As never replaces an existing file.
    pub fn save_to(&mut self, path: &Path, save_as: bool) -> Result<(), String> {
        if save_as || self.save_path.as_deref() != Some(path) {
            match fs::symlink_metadata(path) {
                Ok(_) => {
                    return Err("Save As requires a new path; existing files are preserved.".into());
                }
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(error) => return Err(format!("save {}: {error}", path.display())),
            }
        }
        self.check_output_path(path)?;
        if let Some(document) = &mut self.imported {
            imported_scene_io::save(path, document.snapshot(), &self.policy)
                .map_err(|e| e.to_string())?;
            document.mark_saved();
        } else {
            scene_io::save(path, self.document.scene(), &self.policy).map_err(|e| e.to_string())?;
            self.document.mark_saved();
        }
        self.save_path = Some(path.to_path_buf());
        self.scene_text = path.display().to_string();
        Ok(())
    }

    fn save_current(&mut self) -> Result<(), String> {
        if let Some(path) = self.save_path.clone() {
            self.save_to(&path, false)
        } else if !self.scene_text.trim().is_empty() {
            self.save_to(&PathBuf::from(self.scene_text.trim()), true)
        } else {
            Err("Enter a new independent .ouscene or .ouimport path before saving.".into())
        }
    }

    fn request(&mut self, action: Pending) -> Result<(), String> {
        self.check_idle()?;
        if self.is_dirty() {
            self.pending = Some(action);
            Ok(())
        } else {
            self.perform(action)
        }
    }

    /// Start a new document, guarded against discarding unsaved work.
    pub fn request_new(&mut self) -> Result<(), String> {
        self.request(Pending::New)
    }

    /// Validate a replacement before prompting to discard the current document.
    pub fn request_open(&mut self, path: &Path) -> Result<(), String> {
        self.check_idle()?;
        let candidate = self.load_document(path)?;
        self.protect_candidate(&candidate);
        self.request(candidate)
    }

    /// Request closing through the same unsaved document guard.
    pub fn request_close(&mut self) -> Result<(), String> {
        self.request(Pending::Close)
    }

    /// Resolve an unsaved guard. Save failure preserves both work and the guard.
    pub fn resolve_unsaved(&mut self, choice: UnsavedChoice) -> Result<(), String> {
        if self.pending.is_none() {
            return Ok(());
        }
        if matches!(choice, UnsavedChoice::Cancel) {
            self.pending = None;
            return Ok(());
        }
        if matches!(choice, UnsavedChoice::Save) {
            self.save_current()?;
        }
        if let Some(action) = self.pending.take() {
            self.perform(action)?;
        }
        Ok(())
    }

    fn perform(&mut self, action: Pending) -> Result<(), String> {
        let replacing = !matches!(action, Pending::Close);
        let next_scope = if replacing {
            self.inspector_scope
                .checked_add(1)
                .ok_or("Document identity space exhausted.")?
        } else {
            self.inspector_scope
        };
        match action {
            Pending::New => {
                self.imported = None;
                self.document = Document::new(
                    Scene::new("Untitled").map_err(|e| e.to_string())?,
                    self.preferences.history_capacity,
                );
                self.save_path = None;
                self.scene_text.clear();
            }
            Pending::Open(scene, path) => {
                self.imported = None;
                self.document.replace(scene);
                self.scene_text = path.display().to_string();
                self.save_path = Some(path);
            }
            Pending::Imported(document, path) => {
                self.imported = Some(*document);
                self.scene_text = path
                    .as_ref()
                    .map_or_else(String::new, |p| p.display().to_string());
                self.save_path = path;
            }
            Pending::Close => self.close_allowed = true,
        }
        if replacing {
            self.inspector_scope = next_scope;
            self.reconcile_inspector();
        }
        self.inspector_panel = InspectorPanel::default();
        self.imported_inspector = imported::InspectorPanel::default();
        self.imported_viewport = imported::ViewportPanel::default();
        Ok(())
    }

    /// Execute a panel transaction against the one live document and history.
    pub fn apply_action(&mut self, action: EditorAction) -> Result<(), String> {
        self.check_idle()?;
        if self.imported.is_some() {
            return match action {
                EditorAction::Select { id, toggle } => self.apply_imported_action(imported::ImportedAction::Select { id: Some(id), toggle }),
                _ => Err("Structural, primitive, Euler and generic authoring edits are unavailable for imported documents.".into()),
            };
        }
        match action {
            EditorAction::Select { id, toggle } => {
                let (scene, selection) = self.document.selection_mut();
                if toggle {
                    selection.toggle(id, scene)
                } else {
                    selection.replace(Some(id), scene)
                }
                .map_err(|e| e.to_string())?;
            }
            EditorAction::Create { primitive, parent } => {
                let mut created = None;
                self.document
                    .edit("Create entity", |scene| {
                        created =
                            Some(scene.create(format!("{primitive:?}"), primitive, parent)?);
                        Ok(())
                    })
                    .map_err(|e| e.to_string())?;
                let (scene, selection) = self.document.selection_mut();
                selection
                    .replace(created, scene)
                    .map_err(|e| e.to_string())?;
            }
            EditorAction::DeleteSelection => {
                let ids: Vec<_> = self.document.selection().ids().collect();
                self.document
                    .edit("Delete selection", |scene| {
                        for id in ids {
                            if scene.get(id).is_some() {
                                scene.remove_subtree(id)?;
                            }
                        }
                        Ok(())
                    })
                    .map_err(|e| e.to_string())?;
            }
            EditorAction::Edit {
                id,
                name,
                active,
                transform,
            } => {
                self.document
                    .edit("Edit entity", |scene| {
                        scene.edit(id, |entity| {
                            entity.name = name;
                            entity.active = active;
                            entity.transform = transform;
                        })
                    })
                    .map_err(|e| e.to_string())?;
            }
            EditorAction::Reparent { id, parent } => {
                self.document
                    .edit("Reparent entity", |scene| {
                        scene.edit(id, |entity| entity.parent = parent)
                    })
                    .map_err(|e| e.to_string())?;
            }
        }
        self.reconcile_inspector();
        Ok(())
    }

    fn report(&mut self, result: Result<(), String>, success: &str) {
        let (severity, message) = match result {
            Ok(()) => (Severity::Info, success.to_owned()),
            Err(error) => (Severity::Error, error),
        };
        self.status = message.clone();
        if let Err(error) = self.console.push(severity, &message) {
            self.status = format!("{message}; console recording failed: {error}");
        }
    }

    fn command(&mut self, command: Command) {
        let availability = shortcuts::StateAvailability {
            can_undo: self.can_undo(),
            can_redo: self.can_redo(),
            has_selection: self.imported.as_ref().map_or_else(
                || self.document.selection().primary().is_some(),
                |d| d.primary().is_some(),
            ),
            has_save_path: self.save_path.is_some() || !self.scene_text.trim().is_empty(),
        };
        let result = availability
            .availability(command)
            .map_err(str::to_owned)
            .and_then(|()| match command {
                Command::NewScene => self.request_new(),
                Command::OpenScene => self.request_open(&PathBuf::from(self.scene_text.trim())),
                Command::Save => self.save_current(),
                Command::Undo => self.undo().map(|_| ()),
                Command::Redo => self.redo().map(|_| ()),
                Command::DeleteSelection => self.apply_action(EditorAction::DeleteSelection),
                Command::ClearSelection => {
                    if let Some(document) = &mut self.imported {
                        document.select(None, false).map_err(|e| e.to_string())?;
                    } else {
                        self.document.selection_mut().1.clear();
                    }
                    self.reconcile_inspector();
                    Ok(())
                }
            });
        self.report(result, "Document command completed.");
    }

    fn shortcuts(&mut self, ctx: &Context) {
        if self.pending.is_some() || self.path_form.is_some() {
            return;
        }
        let text_focus = ctx.wants_keyboard_input();
        // Leave Escape available for egui to dismiss this actual open popup, but
        // do not also interpret that dismissal as a global selection command.
        let inspector_menu_open = self.workspace.profile() == AppearanceProfile::Reference
            && self
                .inspector_menu_popup
                .is_some_and(|id| egui::Popup::is_id_open(ctx, id));
        let commands = ctx.input(|input| {
            input
                .events
                .iter()
                .filter_map(|event| {
                    let egui::Event::Key {
                        key,
                        pressed: true,
                        repeat: false,
                        modifiers,
                        ..
                    } = event
                    else {
                        return None;
                    };
                    if *key == egui::Key::Escape && inspector_menu_open {
                        return None;
                    }
                    let key = match key {
                        egui::Key::N => Key::N,
                        egui::Key::O => Key::O,
                        egui::Key::S => Key::S,
                        egui::Key::Z => Key::Z,
                        egui::Key::Y => Key::Y,
                        egui::Key::Delete => Key::Delete,
                        egui::Key::Escape => Key::Escape,
                        _ => return None,
                    };
                    shortcuts::resolve(
                        Chord {
                            key,
                            modifiers: Modifiers {
                                ctrl: modifiers.ctrl,
                                command: modifiers.command,
                                shift: modifiers.shift,
                                alt: modifiers.alt,
                            },
                        },
                        text_focus,
                    )
                })
                .collect::<Vec<_>>()
        });
        for command in commands {
            match command {
                Command::OpenScene => self.open_form(PathFormKind::OpenScene),
                Command::Save if self.save_path.is_none() => self.open_form(PathFormKind::SaveAs),
                _ => self.command(command),
            }
        }
    }

    fn menu_item(&mut self, ui: &mut egui::Ui, label: &str, role: &str, enabled: bool) -> bool {
        let response = ui.add_enabled(enabled, egui::Button::new(label));
        self.workspace.record(role, &response, false);
        if response.clicked() {
            ui.close();
            true
        } else {
            false
        }
    }

    fn open_form(&mut self, kind: PathFormKind) {
        if self.pending.is_some() || self.path_form.is_some() {
            return;
        }
        let primary = match kind {
            PathFormKind::OpenProject => self.project_text.clone(),
            PathFormKind::OpenScene | PathFormKind::Import => String::new(),
            PathFormKind::SaveAs => String::new(),
        };
        self.path_form = Some(PathForm {
            kind,
            primary,
            bindings: self.bindings_text.clone(),
            error: None,
        });
    }

    fn toolbar(&mut self, ctx: &Context) {
        let enabled = self.pending.is_none() && self.path_form.is_none();
        let reference = self.workspace.profile() == AppearanceProfile::Reference;
        let project_name = self
            .project
            .as_ref()
            .and_then(|p| p.root().file_name())
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| "OpenUnity".into());
        let toolbar=egui::TopBottomPanel::top("workspace.toolbar").exact_height(30.0)
            .frame(if reference {egui::Frame::NONE.fill(appearance::CHROME)} else {egui::Frame::NONE}).show(ctx, |ui| {
            ui.add_enabled_ui(enabled, |ui| { ui.horizontal_centered(|ui| {
                ui.add_space(5.0);
                let file=ui.menu_button("File", |ui| {
                    if self.menu_item(ui,"New","command.new",true) {self.command(Command::NewScene);}
                    if self.menu_item(ui,"Open Project…","command.open_project",true) {self.open_form(PathFormKind::OpenProject);}
                    if self.menu_item(ui,"Open Scene/Archive…","command.open_scene",true) {self.open_form(PathFormKind::OpenScene);}
                    if self.menu_item(ui,"Import Saved Scene…","command.import",self.project.is_some()) {self.open_form(PathFormKind::Import);}
                    ui.separator();
                    if self.menu_item(ui,"Save","command.save",true) {
                        if self.save_path.is_some() {let result=self.save_current();self.report(result,"Scene saved.");}
                        else {self.open_form(PathFormKind::SaveAs);}
                    }
                    if self.menu_item(ui,"Save As…","command.save_as",true) {self.open_form(PathFormKind::SaveAs);}
                    ui.separator();
                    if self.menu_item(ui,"Close","command.close",true) {let result=self.request_close();self.report(result,"Close requested.");}
                });
                self.workspace.record("menu.file",&file.response,false);
                let edit=ui.menu_button("Edit",|ui| {
                    if self.menu_item(ui,"Undo","command.undo",self.can_undo()) {self.command(Command::Undo);}
                    if self.menu_item(ui,"Redo","command.redo",self.can_redo()) {self.command(Command::Redo);}
                });
                self.workspace.record("menu.edit",&edit.response,false);
                let view=ui.menu_button("View",|ui| {
                    if self.menu_item(ui,"Reset Workspace","command.reset",true) {self.workspace.reset();}
                    let grid=ui.checkbox(&mut self.preferences.grid_visible,"World grid");
                    self.workspace.record("command.grid",&grid,self.preferences.grid_visible);
                    if reference && self.menu_item(ui,"Build…","command.build",true) { self.report(capabilities::attempt(Capability::Build).map_err(|e|e.to_string()),""); }
                });
                self.workspace.record("menu.view",&view.response,false);
                if !reference {ui.separator();
                for (role,capability) in [("command.play",Capability::Play),("command.build",Capability::Build)] {
                    let response=ui.button(capability.label()).on_hover_text("Unavailable in this independent authoring editor; click for the capability diagnostic.");
                    self.workspace.record(role,&response,false);
                    if response.clicked() {self.report(capabilities::attempt(capability).map_err(|e|e.to_string()),"");}
                }
                ui.separator();
                ui.weak("OpenUnity");} else {ui.add_space(8.0);ui.add_sized([160.0,18.0],egui::Label::new(egui::RichText::new(&project_name).color(appearance::MUTED)).truncate()).on_hover_text(&project_name);}
            });});
            if reference {
                let rect=ui.max_rect();
                let mut transport=ui.new_child(egui::UiBuilder::new().id_salt("workspace.transport").max_rect(egui::Rect::from_center_size(rect.center(),egui::vec2(94.0,22.0))).layout(egui::Layout::left_to_right(egui::Align::Center)));
                transport.spacing_mut().item_spacing.x=1.0;
                for (role,label,reason) in [("command.play","Play","Play execution is unsupported."),("command.pause","Pause","Pause requires unsupported Play execution."),("command.step","Step","Step requires unsupported Play execution.")] {
                    let response=transport.add_enabled(false,egui::Button::new("").min_size(egui::vec2(29.0,20.0))).on_disabled_hover_text(reason);
                    response.widget_info(||egui::WidgetInfo::labeled(egui::WidgetType::Button,false,label));
                    let icon_role=match role {"command.play"=>"transport_play","command.pause"=>"transport_pause",_=>"transport_step"};
                    if !crate::resources::paint(&transport,icon_role,role,response.id,egui::Rect::from_center_size(response.rect.center(),egui::vec2(16.0,16.0))) {
                        let r=response.rect.shrink(6.0);
                        let painter=transport.painter();
                        if role=="command.pause" {for x in [r.left()+1.0,r.right()-4.0]{painter.rect_filled(egui::Rect::from_min_size(egui::pos2(x,r.top()),egui::vec2(3.0,r.height())),0.0,appearance::MUTED);}}
                        else {painter.add(egui::Shape::convex_polygon(vec![r.left_top(),egui::pos2(r.right()-if role=="command.step"{3.0}else{0.0},r.center().y),r.left_bottom()],appearance::MUTED,egui::Stroke::NONE));if role=="command.step"{painter.line_segment([r.right_top(),r.right_bottom()],egui::Stroke::new(2.0,appearance::MUTED));}}
                    }
                    self.workspace.record(role,&response,false);
                }
                let mut right=ui.new_child(egui::UiBuilder::new().id_salt("workspace.layout_menu").max_rect(egui::Rect::from_min_max(egui::pos2(rect.right()-110.0,rect.top()),rect.max)).layout(egui::Layout::right_to_left(egui::Align::Center)));
                right.add_enabled_ui(enabled,|ui|{let menu=ui.menu_button("Layout",|ui| {if self.menu_item(ui,"Reset Workspace","command.reset",true){self.workspace.reset();}});self.workspace.record("menu.layout",&menu.response,false);});
            }
        });
        self.workspace.toolbar_rect(toolbar.response.rect);
        let name = self.imported.as_ref().map_or_else(
            || self.document.scene().name(),
            |d| d.plan().scene_name.as_str(),
        );
        let project = self.project.as_ref().map_or_else(
            || "No project".to_owned(),
            |p| p.root().display().to_string(),
        );
        let identity = format!(
            "{} · {} · {name} · {project}",
            self.status,
            if self.is_dirty() { "Modified" } else { "Saved" }
        );
        let small = ctx.content_rect().width() < 875.0 || ctx.content_rect().height() < 543.0;
        let status = egui::TopBottomPanel::bottom("workspace.status")
            .exact_height(20.0)
            .frame(if reference {
                egui::Frame::NONE.fill(appearance::CHROME)
            } else {
                egui::Frame::NONE
            })
            .show(ctx, |ui| {
                ui.horizontal_centered(|ui| {
                    ui.add_space(5.0);
                    let text = if small {
                        format!("Workspace below 875×543; scroll to reach panes. {identity}")
                    } else {
                        identity.clone()
                    };
                    ui.add(egui::Label::new(text).truncate())
                        .on_hover_text(&identity);
                });
            });
        self.workspace.status_rect(status.response.rect);
    }

    /// Render the production Reference Inspector action area inside an allocated header.
    /// The workspace reserves this area outside the tab-title hit rectangle.
    /// Returns activation requests for ordered dispatch against committed state.
    pub fn show_inspector_header(&mut self, ui: &mut egui::Ui) -> Vec<InspectorHeaderAction> {
        if self.workspace.profile() != AppearanceProfile::Reference {
            return Vec::new();
        }
        let mut requests = Vec::new();
        ui.spacing_mut().item_spacing.x = 0.0;
        ui.spacing_mut().button_padding = egui::Vec2::ZERO;
        let available = self.inspector_controller.snapshot().available
            && self.pending.is_none()
            && self.path_form.is_none();
        let response = ui
            .add_enabled_ui(available, |ui| {
                let response = ui.add_sized(
                    [24.0, 24.0],
                    egui::Button::new("").selected(self.inspector_controller.snapshot().locked),
                );
                if response.clicked() && self.inspector_controller.snapshot().available {
                    requests.push(InspectorHeaderAction::ToggleLock);
                    ui.ctx().request_repaint();
                }
                let state = self.inspector_controller.snapshot();
                response.widget_info(|| {
                    egui::WidgetInfo::selected(
                        egui::WidgetType::Checkbox,
                        response.enabled(),
                        state.locked,
                        "Lock Inspector",
                    )
                });
                let icon_rect =
                    egui::Rect::from_center_size(response.rect.center(), egui::vec2(16.0, 16.0));
                crate::resources::paint(
                    ui,
                    if state.locked {
                        "lock_locked"
                    } else {
                        "lock_unlocked"
                    },
                    "inspector.lock",
                    response.id,
                    icon_rect,
                );
                response
            })
            .inner;
        let state = self.inspector_controller.snapshot();
        let icon_rect =
            egui::Rect::from_center_size(response.rect.center(), egui::vec2(16.0, 16.0));
        let mut control =
            crate::inspector_lock::control_json("inspector.lock", &response, state.locked);
        control["icon_rect"] = appearance::rect_json(icon_rect);
        control["accessible_name"] = "Lock Inspector".into();
        control["accessible_pressed"] = state.locked.into();
        control["reason"] = state.reason.clone().into();
        self.inspector_controls.push(control);
        response
            .on_hover_text(if state.locked {
                "Unlock Inspector"
            } else {
                "Lock Inspector"
            })
            .on_disabled_hover_text(
                state
                    .reason
                    .as_deref()
                    .unwrap_or("Inspector actions unavailable while a document dialog is open."),
            );
        let menu_was_open = self
            .inspector_menu_popup
            .is_some_and(|id| egui::Popup::is_id_open(ui.ctx(), id));
        let menu = ui.menu_button("⋮", |ui| {
            if !menu_was_open {
                self.inspector_component_menu_context = self.inspector_target();
            }
            let state = self.inspector_controller.snapshot();
            let available = state.available && self.pending.is_none() && self.path_form.is_none();
            let mut checked = state.locked;
            let response = ui.add_enabled(available, egui::Checkbox::new(&mut checked, "Lock"));
            self.inspector_controls
                .push(crate::inspector_lock::control_json(
                    "inspector.menu.lock",
                    &response,
                    state.locked,
                ));
            let reason = state.reason.clone();
            if response.changed() && self.inspector_controller.snapshot().available {
                requests.push(InspectorHeaderAction::ToggleLock);
                ui.close();
                ui.ctx().request_repaint();
            }
            if let Some(reason) = reason {
                response.on_disabled_hover_text(&reason);
                ui.weak(reason);
            }
            ui.separator();
            let expansion = self.component_expansion.snapshot();
            for (label, role, expanded, enabled) in [
                (
                    "Expand All Components",
                    "inspector.menu.expand_all",
                    true,
                    expansion.expand_all_enabled,
                ),
                (
                    "Collapse All Components",
                    "inspector.menu.collapse_all",
                    false,
                    expansion.collapse_all_enabled,
                ),
            ] {
                let response = ui.add_enabled(available && enabled, egui::Button::new(label));
                self.inspector_controls
                    .push(crate::inspector_lock::control_json(role, &response, false));
                if response.clicked() {
                    if let Some(context) = self.inspector_component_menu_context {
                        requests
                            .push(InspectorHeaderAction::SetAllComponents { context, expanded });
                    }
                    ui.close();
                    ui.ctx().request_repaint();
                }
                if let Some(reason) = &expansion.reason {
                    response.on_disabled_hover_text(reason);
                }
            }
            if let Some(reason) = &expansion.reason {
                ui.weak(reason);
            }
        });
        self.inspector_menu_popup = Some(egui::Popup::default_response_id(&menu.response));
        menu.response.widget_info(|| {
            egui::WidgetInfo::labeled(
                egui::WidgetType::Button,
                menu.response.enabled(),
                "Inspector menu",
            )
        });
        self.inspector_controls
            .push(crate::inspector_lock::control_json(
                "inspector.menu",
                &menu.response,
                false,
            ));
        requests
    }

    fn panels(&mut self, ctx: &Context) {
        let mut actions = Vec::new();
        let mut imported_actions = Vec::new();
        enum Dispatch {
            Authoring(EditorAction),
            Imported(imported::ImportedAction),
            Inspector(InspectorTarget, EditorAction),
            ImportedInspector(InspectorTarget, imported::ImportedAction),
            ToggleInspectorLock,
            SetAllComponents(InspectorTarget, bool),
            SetComponent(InspectorTarget, i64, bool),
        }
        let mut ordered = Vec::new();
        self.reconcile_inspector();
        self.inspector_controls.clear();
        let enabled = self.pending.is_none() && self.path_form.is_none();
        let reference = self.workspace.profile() == AppearanceProfile::Reference;
        self.viewport_panel
            .set_grid_visible(self.preferences.grid_visible);
        self.imported_viewport
            .set_grid_visible(self.preferences.grid_visible);
        let mut placeholder = Workspace::default();
        placeholder.set_profile(self.workspace.profile());
        let mut workspace = std::mem::replace(&mut self.workspace, placeholder);
        workspace.show_body(ctx,enabled,|ui,tab,header| {
        if header {
            ordered.extend(self.show_inspector_header(ui).into_iter().map(|request|match request {
                InspectorHeaderAction::ToggleLock=>Dispatch::ToggleInspectorLock,
                InspectorHeaderAction::SetAllComponents { context,expanded }=>Dispatch::SetAllComponents(context,expanded),
            }));
            return;
        }
        match tab {
            TabId::Hierarchy => {
                if reference {
                    if let Some(document)=&self.imported {imported_actions.extend(imported::show_hierarchy_reference(ui,document,&mut self.hierarchy_browser));}
                    else {actions.extend(hierarchy::show_reference(ui,self.document.scene(),self.document.selection(),&mut self.hierarchy_browser));}
                } else {
                egui::ScrollArea::vertical().id_salt("workspace.hierarchy.scroll").show(ui,|ui| {
                    if let Some(document)=&self.imported {imported_actions.extend(imported::show_hierarchy(ui,document));}
                    else {actions.extend(hierarchy::show(ui,self.document.scene(),self.document.selection()));}
                });
                }
            }
            TabId::Inspector => {
                let scroll=egui::ScrollArea::vertical().id_salt("workspace.inspector.scroll");
                let scroll=scroll.show(ui,|ui| {
                    if reference {
                        if let Some(notice) = &self.inspector_controller.snapshot().notice { ui.weak(notice); }
                        let target = self.inspector_target();
                        if let Some(document)=&self.imported {
                            let emitted = self.imported_inspector.show_target(ui, document, target.map(|t| t.entity_id),self.component_expansion.snapshot());
                            for action in emitted {
                                if let imported::ImportedAction::SetComponentExpanded { file_id,expanded } = action { if let Some(target) = target { ordered.push(Dispatch::SetComponent(target,file_id,expanded)); } }
                                else if matches!(action, imported::ImportedAction::Select { .. }) { ordered.push(Dispatch::Imported(action)); }
                                else if let Some(target) = target { ordered.push(Dispatch::ImportedInspector(target,action)); }
                            }
                        } else {
                            let emitted = self.inspector_panel.show_target(ui,self.document.scene(),self.document.selection(),target.map(|t| t.entity_id));
                            if let Some(target) = target { ordered.extend(emitted.into_iter().map(|action| Dispatch::Inspector(target,action))); }
                        }
                        let (owner,changed) = if self.imported.is_some() { self.imported_inspector.draft_state() } else { self.inspector_panel.draft_state() };
                        self.inspector_controller.draft(owner.map(|entity_id| InspectorTarget { document_scope:self.inspector_scope,entity_id }),changed);
                    } else if let Some(document)=&self.imported {imported_actions.extend(self.imported_inspector.show(ui,document));}
                    else {actions.extend(self.inspector_panel.show(ui,self.document.scene(),self.document.selection()));}
                });
                self.inspector_scroll=serde_json::json!({"inner_rect":appearance::rect_json(scroll.inner_rect),"content_rect":appearance::rect_json(egui::Rect::from_min_size(scroll.inner_rect.min-scroll.state.offset,scroll.content_size)),"offset":[scroll.state.offset.x,scroll.state.offset.y]});

            }
            TabId::Scene => {
                if let Some(document)=&self.imported {imported_actions.extend(self.imported_viewport.show(ui,document));}
                else {actions.extend(self.viewport_panel.show(ui,self.document.scene(),self.document.selection()));}
            }
            TabId::Game => {
                if reference {
                    let rect=ui.available_rect_before_wrap();
                    ui.painter().rect_filled(rect.expand2(egui::vec2(4.0,0.0)),0.0,egui::Color32::from_rgb(55,76,118));
                    egui::Frame::NONE.fill(appearance::BAR).inner_margin(4.0).show(ui,|ui|{
                        ui.set_width((rect.width()-8.0).max(1.0));
                        ui.horizontal(|ui|{ui.label("Game rendering unavailable").on_hover_text("Empty design canvas only. No game renderer, camera output or Play execution is available. Switch to Scene for the actual Transform-only preview.");ui.with_layout(egui::Layout::right_to_left(egui::Align::Center),|ui|{ui.add_enabled(false,egui::Button::new("Display")).on_disabled_hover_text("Game displays are unsupported.");});});
                    });
                } else {ui.heading("Game rendering is unsupported");ui.label("This editor provides independent authoring previews. No game renderer or Play execution is available.");}
            }
            TabId::Project => self.project_panel.show(ui,self.project.as_ref()),
            TabId::Console => self.console_panel.show(ui,&mut self.console),
        }
        if reference {
            ordered.extend(imported_actions.drain(..).map(Dispatch::Imported));
            ordered.extend(actions.drain(..).map(Dispatch::Authoring));
        }
        });
        self.workspace = workspace;
        self.preferences.grid_visible = if self.imported.is_some() {
            self.imported_viewport.grid_visible()
        } else {
            self.viewport_panel.grid_visible()
        };
        if enabled {
            for command in ordered {
                let result = match command {
                    Dispatch::Authoring(action) => self.apply_action(action),
                    Dispatch::Imported(action) => self.apply_imported_action(action),
                    Dispatch::Inspector(target, action) => {
                        self.apply_inspector_action(target, action)
                    }
                    Dispatch::ImportedInspector(target, action) => {
                        self.apply_imported_inspector_action(target, action)
                    }
                    Dispatch::ToggleInspectorLock => self.toggle_inspector_lock(),
                    Dispatch::SetAllComponents(context, expanded) => {
                        self.set_all_inspector_components_expanded(context, expanded)
                    }
                    Dispatch::SetComponent(context, file_id, expanded) => {
                        self.set_inspector_component_expanded(context, file_id, expanded)
                    }
                };
                self.report(result, "Document panel action completed.");
            }
            for action in imported_actions {
                let result = self.apply_imported_action(action);
                self.report(result, "Imported document action completed.");
            }
            for action in actions {
                let result = self.apply_action(action);
                self.report(result, "Authoring action completed.");
            }
        }
    }

    fn path_dialog(&mut self, ctx: &Context) {
        if self.pending.is_some() {
            self.path_form = None;
            return;
        }
        let Some(mut form) = self.path_form.take() else {
            return;
        };
        let mut confirm = false;
        let mut cancel = false;
        egui::Modal::new(egui::Id::new("workspace.path_form")).show(ctx,|ui| {
            ui.set_min_width(420.0);
            ui.heading(form.kind.title());
            if matches!(form.kind,PathFormKind::Import) {
                ui.label("Paths are confined to the opened source project. Source files remain read-only.");
            }
            let label=ui.label(match form.kind {
                PathFormKind::OpenProject=>"Source project directory",
                PathFormKind::OpenScene=>"Independent .ouscene / .ouimport path",
                PathFormKind::Import=>"Saved .unity scene path",
                PathFormKind::SaveAs=>if self.imported.is_some(){"New independent .ouimport path"}else{"New independent .ouscene path"},
            });
            let input=ui.text_edit_singleline(&mut form.primary).labelled_by(label.id);
            self.workspace.record("form.primary",&input,false);
            if matches!(form.kind,PathFormKind::Import) {
                let label=ui.label("Explicit script bindings path");
                let input=ui.text_edit_singleline(&mut form.bindings).labelled_by(label.id);
                self.workspace.record("form.bindings",&input,false);
            }
            if let Some(error)=&form.error {ui.colored_label(ui.visuals().error_fg_color,error);}
            ui.horizontal(|ui| {
                let response=ui.button("Confirm");self.workspace.record("form.confirm",&response,false);confirm=response.clicked();
                let response=ui.button("Cancel");self.workspace.record("form.cancel",&response,false);cancel=response.clicked();
            });
        });
        if cancel || ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            return;
        }
        if confirm {
            let result = if form.primary.trim().is_empty() {
                Err("Enter a nonempty path before confirming.".to_owned())
            } else if matches!(form.kind, PathFormKind::Import) && form.bindings.trim().is_empty() {
                Err("Enter an explicit script bindings path.".to_owned())
            } else {
                let path = PathBuf::from(form.primary.trim());
                match form.kind {
                    PathFormKind::OpenProject => self.open_project(&path),
                    PathFormKind::OpenScene => self.request_open(&path),
                    PathFormKind::Import => {
                        self.request_import(&path, &PathBuf::from(form.bindings.trim()))
                    }
                    PathFormKind::SaveAs => self.save_to(&path, true),
                }
            };
            match result {
                Ok(()) => {
                    if matches!(form.kind, PathFormKind::Import) {
                        self.bindings_text = form.bindings;
                    }
                    self.report(Ok(()), "Path command completed.");
                    return;
                }
                Err(error) => {
                    form.error = Some(error.clone());
                    self.report(Err(error), "");
                }
            }
        }
        self.path_form = Some(form);
    }

    fn unsaved_dialog(&mut self, ctx: &Context) {
        if self.pending.is_none() {
            return;
        }
        let mut choice = None;
        egui::Modal::new(egui::Id::new("unsaved_work")).show(ctx, |ui| {
            ui.heading("Save unsaved work?");
            ui.label("Save successfully before continuing, discard changes, or cancel.");
            if self.save_path.is_none() {
                ui.label(if self.imported.is_some() {
                    "New independent .ouimport save path"
                } else {
                    "New independent .ouscene save path"
                });
                ui.text_edit_singleline(&mut self.scene_text);
            }
            ui.label(&self.status);
            ui.horizontal(|ui| {
                if ui.button("Save").clicked() {
                    choice = Some(UnsavedChoice::Save);
                }
                if ui.button("Discard").clicked() {
                    choice = Some(UnsavedChoice::Discard);
                }
                if ui.button("Cancel").clicked() {
                    choice = Some(UnsavedChoice::Cancel);
                }
            });
        });
        // Escape always cancels replacement; it can never discard or save.
        if ctx.input(|input| input.key_pressed(egui::Key::Escape)) {
            choice = Some(UnsavedChoice::Cancel);
        }
        if let Some(choice) = choice {
            let result = self.resolve_unsaved(choice);
            self.report(result, "Unsaved-work decision applied.");
        }
    }

    fn smoke_frame(&mut self, ctx: &Context) {
        let Some(receipt) = &self.smoke else {
            return;
        };
        let Ok(mut status) = receipt.lock() else {
            self.close_allowed = true;
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            return;
        };
        status.frames = self.frames;
        status.appearance_profile = Some(self.workspace.profile().name().to_owned());
        status.workspace_snapshot = Some(self.workspace.snapshot().to_json());
        status.appearance_snapshot = Some(self.appearance_snapshot());
        match self.semantic_snapshot() {
            Ok(snapshot) => status.document_snapshot = Some(snapshot),
            Err(error) => status.error = Some(error),
        }
        ctx.input(|input| {
            for event in &input.events {
                if let egui::Event::Screenshot { image, .. } = event
                    && image.size[0] > 0
                    && image.size[1] > 0
                {
                    status.screenshot_size = Some(image.size);
                    status.screenshot_image = Some((**image).clone());
                }
            }
        });
        if self.frames >= 3 && status.screenshot_size.is_some() {
            status.completed = true;
            self.close_allowed = true;
        } else if self.frames >= 600 || self.smoke_started.elapsed() > Duration::from_secs(20) {
            status.error = Some(
                "Native smoke timed out before a nonempty GPU screenshot was delivered.".into(),
            );
            self.close_allowed = true;
        } else if self.frames >= 3 && !self.smoke_requested {
            ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(egui::UserData::default()));
            self.smoke_requested = true;
        }
        ctx.request_repaint_after(Duration::from_millis(16));
    }

    /// Render one shell frame; also used for headless panel acceptance checks.
    pub fn show(&mut self, ctx: &Context) {
        if let Some(error) = appearance::apply(ctx, self.workspace.profile()) {
            self.report(Err(error), "");
        }
        if self.workspace.profile() == AppearanceProfile::Reference {
            let name = self.imported.as_ref().map_or_else(
                || self.document.scene().name(),
                |d| d.plan().scene_name.as_str(),
            );
            let project = self
                .project
                .as_ref()
                .and_then(|p| p.root().file_name())
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_else(|| "No project".into());
            ctx.send_viewport_cmd(egui::ViewportCommand::Title(format!(
                "{name}{} — {project} — OpenUnity",
                if self.is_dirty() { " *" } else { "" }
            )));
        }
        self.frames = self.frames.saturating_add(1);
        if ctx.input(|input| input.viewport().close_requested())
            && !self.close_allowed
            && self.smoke.is_none()
        {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            let result = self.request_close();
            self.report(result, "Close requested.");
        }
        if let Some(error) = crate::resources::begin_frame(ctx, self.workspace.profile()) {
            self.report(Err(error), "");
        }
        self.workspace.begin_frame(ctx);
        self.shortcuts(ctx);
        self.toolbar(ctx);
        self.panels(ctx);
        self.path_dialog(ctx);
        self.unsaved_dialog(ctx);
        self.resource_snapshot = crate::resources::snapshot(ctx);
        self.smoke_frame(ctx);
        if self.close_allowed {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
    }
}

impl eframe::App for EditorApp {
    fn update(&mut self, ctx: &Context, _frame: &mut eframe::Frame) {
        self.show(ctx);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use openunity_editor_core::scene::Primitive;
    use std::sync::atomic::{AtomicU64, Ordering};
    static NEXT: AtomicU64 = AtomicU64::new(0);
    struct Directory(PathBuf);
    impl Directory {
        fn new() -> Self {
            let path = std::env::temp_dir().canonicalize().unwrap().join(format!(
                "openunity-shell-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir(&path).unwrap();
            Self(path)
        }
    }
    impl Drop for Directory {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn replacement_guard_preserves_work_on_cancel_and_failed_save() {
        let directory = Directory::new();
        let path = directory.0.join("replacement.ouscene");
        let policy = WritePolicy::new([]).unwrap();
        scene_io::save(&path, &Scene::new("Replacement").unwrap(), &policy).unwrap();
        let mut app = EditorApp::new(AppOptions::default()).unwrap();
        app.apply_action(EditorAction::Create {
            primitive: Primitive::Cube,
            parent: None,
        })
        .unwrap();
        let selected = app.document().selection().primary();
        app.request_open(&path).unwrap();
        assert_eq!(app.document().scene().entities().len(), 1);
        app.resolve_unsaved(UnsavedChoice::Cancel).unwrap();
        assert_eq!(app.document().selection().primary(), selected);
        app.request_open(&path).unwrap();
        app.scene_text = directory
            .0
            .join("missing/failed.ouscene")
            .display()
            .to_string();
        assert!(app.resolve_unsaved(UnsavedChoice::Save).is_err());
        assert!(app.document().is_dirty());
        assert_eq!(app.document().scene().entities().len(), 1);
        assert!(app.save_path().is_none());
        app.resolve_unsaved(UnsavedChoice::Discard).unwrap();
        assert_eq!(app.document().scene().name(), "Replacement");
        assert!(app.document().selection().primary().is_none());
        assert!(!app.document().can_undo());
        assert!(!app.document().is_dirty());
    }

    #[test]
    fn save_as_preserves_existing_target_and_save_failure_keeps_current_path() {
        let directory = Directory::new();
        let path = directory.0.join("owned.ouscene");
        let mut app = EditorApp::new(AppOptions::default()).unwrap();
        app.save_to(&path, true).unwrap();
        let before = fs::read(&path).unwrap();
        app.apply_action(EditorAction::Create {
            primitive: Primitive::Camera,
            parent: None,
        })
        .unwrap();
        assert!(app.save_to(&path, true).is_err());
        assert_eq!(fs::read(&path).unwrap(), before);
        let failed = directory.0.join("missing/failure.ouscene");
        assert!(app.save_to(&failed, true).is_err());
        assert_eq!(app.save_path(), Some(path.as_path()));
        assert!(app.document().is_dirty());
        app.save_to(&path, false).unwrap();
        assert!(!app.document().is_dirty());
        assert_eq!(
            scene_io::load(&path, &app.policy).unwrap(),
            *app.document().scene()
        );
    }

    #[test]
    fn escape_cancels_unsaved_close_and_discard_emits_close() {
        let mut app = EditorApp::new(AppOptions::default()).unwrap();
        app.apply_action(EditorAction::Create {
            primitive: Primitive::Cube,
            parent: None,
        })
        .unwrap();
        app.request_close().unwrap();
        let context = Context::default();
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1200.0, 800.0),
            )),
            events: vec![egui::Event::Key {
                key: egui::Key::Escape,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers::NONE,
            }],
            ..Default::default()
        };
        let output = context.run(input, |ctx| app.show(ctx));
        assert!(!output.shapes.is_empty());
        assert!(!output.viewport_output.values().any(|viewport| {
            viewport
                .commands
                .iter()
                .any(|command| matches!(command, egui::ViewportCommand::Close))
        }));
        assert_eq!(app.document().scene().entities().len(), 1);
        app.resolve_unsaved(UnsavedChoice::Discard).unwrap();
        let output = context.run(egui::RawInput::default(), |ctx| app.show(ctx));
        assert!(!output.viewport_output.values().any(|viewport| {
            viewport
                .commands
                .iter()
                .any(|command| matches!(command, egui::ViewportCommand::Close))
        }));
        app.request_close().unwrap();
        app.resolve_unsaved(UnsavedChoice::Discard).unwrap();
        let output = context.run(egui::RawInput::default(), |ctx| app.show(ctx));
        assert!(output.viewport_output.values().any(|viewport| {
            viewport
                .commands
                .iter()
                .any(|command| matches!(command, egui::ViewportCommand::Close))
        }));
    }
}
