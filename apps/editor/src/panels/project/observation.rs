//! Lazy last-frame observation and one-time replacement snapshot freezing.

mod serialization;

use std::path::{Path, PathBuf};
use std::sync::Arc;

use super::cache::CachedEntryIndices;
use super::*;
use openunity_editor_core::project::{ProjectSnapshotId, ProjectSourceKey};

pub(super) struct FrameObservation {
    project_id: Option<ProjectSnapshotId>,
    source: Option<ObservedSource>,
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

#[derive(Clone, Copy)]
enum ObservedSource {
    Assets,
    EmbeddedPackage(usize),
}

impl ObservedSource {
    fn from_key(project: &Project, key: &ProjectSourceKey) -> Option<Self> {
        match key {
            ProjectSourceKey::Assets => Some(Self::Assets),
            ProjectSourceKey::EmbeddedPackage(name) => project
                .packages()
                .binary_search_by(|package| package.name().cmp(name))
                .ok()
                .map(Self::EmbeddedPackage),
        }
    }

    fn resolve(self, project: &Project) -> Option<ProjectSource<'_>> {
        match self {
            Self::Assets => project.source(None),
            Self::EmbeddedPackage(index) => project
                .packages()
                .get(index)
                .and_then(|package| project.source(Some(package.name()))),
        }
    }
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
            source: None,
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
        source: ProjectSource<'_>,
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
            source: ObservedSource::from_key(project, source.key()),
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
        source: ProjectSource<'_>,
        entry_directory: &Path,
        visible_indices: &CachedEntryIndices,
    ) {
        let observation = FrameObservation::reference(
            self,
            layout,
            frame,
            project,
            source,
            entry_directory,
            visible_indices,
        );
        self.last_frame = Some(observation);
        self.frozen_snapshot = None;
    }
}
