//! Read-only project discovery panel; listed files are not imported engine assets.

mod assets;
mod cache;
mod content;
mod folder;
mod legacy;
mod observation;
mod reference;
mod source;
mod source_controls;
mod state;
#[cfg(test)]
mod tests;
mod toolbar;
mod tree;
mod visibility;

use crate::appearance;
use assets::entry_kind;
use cache::ProjectBrowserCache;
use eframe::egui;
use folder::folder_row;
use openunity_editor_core::project::{
    Project, ProjectEntry, ProjectEntryKind, ProjectSource, ProjectSourceKey,
};
use std::path::{Path, PathBuf};

/// Displays the project reader's bounded snapshot without changing project files.
pub struct ProjectBrowserPanel {
    filter: String,
    project_root: Option<PathBuf>,
    source_key: ProjectSourceKey,
    show_hidden: bool,
    selected_path: Option<PathBuf>,
    selected_path_kind: Option<ProjectEntryKind>,
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

impl Default for ProjectBrowserPanel {
    fn default() -> Self {
        Self {
            filter: String::new(),
            project_root: None,
            source_key: ProjectSourceKey::Assets,
            show_hidden: false,
            selected_path: None,
            selected_path_kind: None,
            directory: PathBuf::new(),
            query_cache: ProjectBrowserCache::default(),
            last_frame: None,
            frozen_snapshot: None,
        }
    }
}

/// Actual body allocations shared by the tree, content and footer.
struct ReferenceLayout {
    available: egui::Rect,
    body: egui::Rect,
    tree_rect: egui::Rect,
    content_rect: egui::Rect,
}
