//! Lazy last-frame observation and one-time replacement snapshot freezing.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use super::cache::CachedEntryIndices;
use super::*;
use openunity_editor_core::project::ProjectSnapshotId;

pub(super) struct FrameObservation {
    project_id: Option<ProjectSnapshotId>,
    folder: PathBuf,
    query: String,
    selected_path: Option<PathBuf>,
    visible_entries: Option<VisibleEntryIndices>,
    tree_rect: serde_json::Value,
    content_rect: serde_json::Value,
    controls: Vec<serde_json::Value>,
    rendered_folders: Vec<serde_json::Value>,
    geometry_truncated: bool,
    search_geometry: serde_json::Value,
    create_icon: serde_json::Value,
}

enum VisibleEntryIndices {
    AllChildren(PathBuf),
    Indices(Arc<[usize]>),
}

impl ReferenceFrame {
    pub(super) fn new() -> Self {
        Self {
            controls: Vec::new(),
            rendered_folders: Vec::new(),
            geometry_truncated: false,
            search_geometry: serde_json::Value::Null,
            create_icon: serde_json::Value::Null,
        }
    }
}

impl FrameObservation {
    fn missing(
        panel: &ProjectBrowserPanel,
        layout: &ReferenceLayout,
        frame: ReferenceFrame,
    ) -> Self {
        Self {
            project_id: None,
            folder: PathBuf::new(),
            query: panel.filter.clone(),
            selected_path: None,
            visible_entries: None,
            tree_rect: appearance::rect_json(layout.tree_rect),
            content_rect: appearance::rect_json(layout.content_rect),
            controls: frame.controls,
            rendered_folders: frame.rendered_folders,
            geometry_truncated: frame.geometry_truncated,
            search_geometry: frame.search_geometry,
            create_icon: frame.create_icon,
        }
    }

    fn reference(
        panel: &ProjectBrowserPanel,
        layout: &ReferenceLayout,
        frame: ReferenceFrame,
        project: &Project,
        entry_directory: &Path,
        visible_indices: &CachedEntryIndices,
    ) -> Self {
        let visible_entries = match visible_indices {
            CachedEntryIndices::AllChildren => {
                VisibleEntryIndices::AllChildren(entry_directory.to_path_buf())
            }
            CachedEntryIndices::Indices(indices) => {
                VisibleEntryIndices::Indices(Arc::clone(indices))
            }
            CachedEntryIndices::AllEntries => {
                VisibleEntryIndices::Indices(Arc::from(Vec::<usize>::new()))
            }
        };

        Self {
            project_id: Some(project.snapshot_id()),
            folder: panel.directory.clone(),
            query: panel.filter.clone(),
            selected_path: panel.selected_path.clone(),
            visible_entries: Some(visible_entries),
            tree_rect: appearance::rect_json(layout.tree_rect),
            content_rect: appearance::rect_json(layout.content_rect),
            controls: frame.controls,
            rendered_folders: frame.rendered_folders,
            geometry_truncated: frame.geometry_truncated,
            search_geometry: frame.search_geometry,
            create_icon: frame.create_icon,
        }
    }

    fn to_json(&self, project: Option<&Project>) -> Option<serde_json::Value> {
        let visible_entries = self.visible_entry_values(project)?;
        let folder = self.folder.to_string_lossy();
        let selected_path = self
            .selected_path
            .as_ref()
            .map(|path| path.to_string_lossy());

        Some(serde_json::json!({
            "folder": folder,
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

        let indices = match visible_entries {
            VisibleEntryIndices::AllChildren(directory) => project.children(directory)?,
            VisibleEntryIndices::Indices(indices) => indices,
        };

        indices
            .iter()
            .map(|&index| {
                let entry = project.entries().get(index)?;
                Some(serde_json::json!({
                    "path": entry.relative_path.to_string_lossy(),
                    "kind": entry_kind(entry.kind)
                }))
            })
            .collect()
    }
}

impl ProjectBrowserPanel {
    /// Actual reference browser state and live controls from the last frame.
    pub fn snapshot(&self, project: Option<&Project>) -> serde_json::Value {
        if let Some(frozen_snapshot) = &self.frozen_snapshot {
            return frozen_snapshot.clone();
        }

        self.last_frame
            .as_ref()
            .and_then(|frame| frame.to_json(project))
            .unwrap_or(serde_json::Value::Null)
    }

    pub(crate) fn freeze_snapshot(&mut self, project: Option<&Project>) {
        if self.frozen_snapshot.is_some() {
            return;
        }

        let Some(frame) = &self.last_frame else {
            return;
        };
        let Some(snapshot) = frame.to_json(project) else {
            return;
        };

        self.query_cache.clear();
        self.frozen_snapshot = Some(snapshot);
    }

    pub(super) fn record_missing_project(
        &mut self,
        layout: &ReferenceLayout,
        frame: ReferenceFrame,
    ) {
        self.query_cache.clear();
        let observation = FrameObservation::missing(self, layout, frame);
        self.last_frame = Some(observation);
        self.frozen_snapshot = None;
    }

    pub(super) fn record_reference(
        &mut self,
        layout: &ReferenceLayout,
        frame: ReferenceFrame,
        project: &Project,
        entry_directory: &Path,
        visible_indices: &CachedEntryIndices,
    ) {
        let observation = FrameObservation::reference(
            self,
            layout,
            frame,
            project,
            entry_directory,
            visible_indices,
        );
        self.last_frame = Some(observation);
        self.frozen_snapshot = None;
    }
}
