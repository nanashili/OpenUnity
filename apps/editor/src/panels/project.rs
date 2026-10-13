//! Read-only project discovery panel; listed files are not imported engine assets.

mod assets;
mod cache;
mod content;
mod folder;
mod legacy;
mod observation;
mod reference;
mod state;
#[cfg(test)]
mod tests;
mod toolbar;
mod tree;

use crate::appearance;
use assets::entry_kind;
use cache::ProjectBrowserCache;
use eframe::egui;
use folder::folder_row;
use openunity_editor_core::project::{Project, ProjectEntry, ProjectEntryKind};
use std::path::{Path, PathBuf};

/// Displays the project reader's bounded snapshot without changing project files.
#[derive(Default)]
pub struct ProjectBrowserPanel {
    filter: String,
    project_root: Option<PathBuf>,
    selected_path: Option<PathBuf>,
    directory: PathBuf,
    query_cache: ProjectBrowserCache,
    last_frame: Option<observation::FrameObservation>,
    frozen_snapshot: Option<serde_json::Value>,
}

/// Per-frame observations, independent of persistent browser selection.
struct ReferenceFrame {
    controls: Vec<serde_json::Value>,
    rendered_folders: Vec<serde_json::Value>,
    geometry_truncated: bool,
    search_geometry: serde_json::Value,
    create_icon: serde_json::Value,
}

/// Actual body allocations shared by the tree, content and footer.
struct ReferenceLayout {
    available: egui::Rect,
    body: egui::Rect,
    tree_rect: egui::Rect,
    content_rect: egui::Rect,
}
