//! Actual browser controls, folder geometry and completed frame snapshots.

use super::*;

impl ReferenceFrame {
    pub(super) fn new() -> Self {
        let controls = Vec::new();
        let rendered_folders = Vec::new();
        let geometry_truncated = false;
        let search_geometry = serde_json::Value::Null;
        let create_icon = serde_json::Value::Null;
        Self {
            controls,
            rendered_folders,
            geometry_truncated,
            search_geometry,
            create_icon,
        }
    }
}

impl ProjectBrowserPanel {
    /// Actual reference browser state and live controls from the last frame.
    pub fn snapshot(&self) -> serde_json::Value {
        self.snapshot.clone()
    }

    pub(super) fn record_missing_project(
        &mut self,
        layout: &ReferenceLayout,
        frame: ReferenceFrame,
    ) {
        self.snapshot = serde_json::json!({
            "folder": "",
            "query": self.filter,
            "selected_path": null,
            "visible_entries": [],
            "tree_rect": appearance::rect_json(layout.tree_rect),
            "content_rect": appearance::rect_json(layout.content_rect),
            "controls": frame.controls,
            "rendered_folders": frame.rendered_folders,
            "geometry_truncated": frame.geometry_truncated,
            "search_geometry": frame.search_geometry,
            "create_icon": frame.create_icon
        });
    }

    pub(super) fn record_reference(
        &mut self,
        layout: &ReferenceLayout,
        frame: ReferenceFrame,
        entries: &[&ProjectEntry],
    ) {
        self.snapshot = serde_json::json!({
            "folder": self.directory.to_string_lossy(),
            "query": self.filter,
            "selected_path": self.selected_path.as_ref().map(|p| p.to_string_lossy()),
            "visible_entries": entries.iter().map(|e| serde_json::json!({
                "path": e.relative_path.to_string_lossy(),
                "kind": entry_kind(e.kind)
            })).collect::<Vec<_>>(),
            "tree_rect": appearance::rect_json(layout.tree_rect),
            "content_rect": appearance::rect_json(layout.content_rect),
            "controls": frame.controls,
            "rendered_folders": frame.rendered_folders,
            "geometry_truncated": frame.geometry_truncated,
            "search_geometry": frame.search_geometry,
            "create_icon": frame.create_icon
        });
    }
}
