//! Materializes the lazily retained observation against its matching snapshot.

use openunity_editor_core::project::Project;

use super::super::{entry_kind, source};
use super::{FrameObservation, VisibleEntryIndices};

impl FrameObservation {
    pub(super) fn to_json(&self, project: Option<&Project>) -> Option<serde_json::Value> {
        let visible_entries = self.visible_entry_values(project)?;
        let source_key = self
            .source
            .and_then(|source| project.and_then(|project| source.resolve(project)))
            .map(|source| source::observation_key(source.key()));
        let folder = self.folder.to_string_lossy();
        let selected_path = self
            .selected_path
            .as_ref()
            .map(|path| path.to_string_lossy());

        Some(serde_json::json!({
            "folder": folder,
            "source": source_key,
            "query": self.query,
            "selected_path": selected_path,
            "visible_entries": visible_entries,
            "tree_rect": self.tree_rect,
            "content_rect": self.content_rect,
            "controls": self.controls,
            "rendered_folders": self.rendered_folders,
            "geometry_truncated": self.geometry_truncated,
            "search_geometry": self.search_geometry,
            "create_icon": self.create_icon
        }))
    }

    fn visible_entry_values(&self, project: Option<&Project>) -> Option<Vec<serde_json::Value>> {
        let Some(visible_entries) = &self.visible_entries else {
            return Some(Vec::new());
        };
        let project = project?;
        let project_id = self.project_id.as_ref()?;

        if project.snapshot_id() != *project_id {
            return None;
        }

        let observed_source = self.source?;
        let source = observed_source.resolve(project)?;

        let indices = match visible_entries {
            VisibleEntryIndices::AllChildren(directory) => source.children(directory)?,
            VisibleEntryIndices::Indices(indices) => indices,
        };

        indices
            .iter()
            .map(|&index| {
                let entry = source.entries().get(index)?;
                Some(serde_json::json!({
                    "path": entry.relative_path.to_string_lossy(),
                    "source_path": source::logical_path(source.key(), &entry.relative_path),
                    "kind": entry_kind(entry.kind)
                }))
            })
            .collect()
    }
}
