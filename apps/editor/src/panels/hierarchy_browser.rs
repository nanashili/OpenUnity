//! Cached, searchable Reference hierarchy over document-local identities.

mod model;
mod projection;
mod row_render;
mod search;
mod sibling_controls;
mod snapshot;
mod tree;

use eframe::egui;
use openunity_editor_core::{hierarchy::HierarchyRevision, scene::EntityId};
use std::collections::HashSet;

pub(crate) use model::{BrowserAction, Row};
use model::{CachedForest, CachedRow};

/// Search state and the current in-memory hierarchy projection.
pub struct HierarchyBrowser {
    query: String,
    lowercase_query: String,
    source: Option<CachedForest>,
    search_included: Option<Vec<bool>>,
    visible_indices: Option<Vec<usize>>,
    collapsed_branches: HashSet<EntityId>,
    collapsed_rows: Vec<bool>,
    scene_expanded: bool,
    selected_ids: Vec<EntityId>,
    search_controls: Vec<serde_json::Value>,
    row_controls: Vec<serde_json::Value>,
    row_geometry: Vec<snapshot::RowGeometry>,
    geometry_truncated: bool,
    content_rect: Option<egui::Rect>,
}

impl Default for HierarchyBrowser {
    fn default() -> Self {
        Self {
            query: String::new(),
            lowercase_query: String::new(),
            source: None,
            search_included: None,
            visible_indices: None,
            collapsed_branches: HashSet::new(),
            collapsed_rows: Vec::new(),
            scene_expanded: true,
            selected_ids: Vec::new(),
            search_controls: Vec::new(),
            row_controls: Vec::new(),
            row_geometry: Vec::new(),
            geometry_truncated: false,
            content_rect: None,
        }
    }
}

impl HierarchyBrowser {
    /// Replace cached rows only when the document publishes new row content.
    pub(crate) fn refresh(
        &mut self,
        revision: HierarchyRevision,
        build: impl FnOnce() -> Result<Vec<Row>, String>,
    ) -> Result<(), String> {
        if self
            .source
            .as_ref()
            .is_some_and(|source| source.revision == revision)
        {
            return Ok(());
        }

        let candidate = build().and_then(|rows| CachedForest::from_rows(revision, rows));
        let candidate = match candidate {
            Ok(candidate) => candidate,
            Err(error) => {
                self.clear_content_cache();
                return Err(error);
            }
        };

        self.collapsed_branches
            .retain(|id| candidate.branches.contains(id));
        self.collapsed_rows.clear();
        self.collapsed_rows.extend(
            candidate
                .rows
                .iter()
                .map(|row| self.collapsed_branches.contains(&row.id)),
        );
        self.source = Some(candidate);
        self.search_included = None;
        self.visible_indices = None;
        self.clear_frame_geometry();
        Ok(())
    }

    /// Release document-derived rows and view state when the active document changes.
    pub fn reset_document(&mut self) {
        self.query = String::new();
        self.lowercase_query = String::new();
        self.source = None;
        self.search_included = None;
        self.visible_indices = None;
        self.collapsed_branches = HashSet::new();
        self.collapsed_rows = Vec::new();
        self.scene_expanded = true;
        self.selected_ids = Vec::new();
        self.search_controls = Vec::new();
        self.release_frame_geometry();
    }

    fn clear_content_cache(&mut self) {
        self.source = None;
        self.search_included = None;
        self.visible_indices = None;
        self.collapsed_branches = HashSet::new();
        self.collapsed_rows = Vec::new();
        self.scene_expanded = true;
        self.selected_ids = Vec::new();
        self.release_frame_geometry();
    }

    fn clear_frame_geometry(&mut self) {
        self.row_controls.clear();
        self.row_geometry.clear();
        self.geometry_truncated = false;
        self.content_rect = None;
    }

    fn release_frame_geometry(&mut self) {
        self.row_controls = Vec::new();
        self.row_geometry = Vec::new();
        self.geometry_truncated = false;
        self.content_rect = None;
    }
}
