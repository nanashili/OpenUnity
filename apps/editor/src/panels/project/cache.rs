//! Per-snapshot query results shared by the virtualized Project browser views.

use std::borrow::Cow;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use openunity_editor_core::project::{Project, ProjectSnapshotId};
use openunity_editor_core::search::ranked_indices;

#[derive(Clone)]
pub(super) enum CachedEntryIndices {
    AllEntries,
    AllChildren,
    Indices(Arc<[usize]>),
}

impl CachedEntryIndices {
    pub(super) fn reference_slice<'a>(
        &'a self,
        project: &'a Project,
        directory: &Path,
    ) -> &'a [usize] {
        match self {
            Self::AllChildren => project.children(directory).unwrap_or(&[]),
            Self::Indices(indices) => indices,
            Self::AllEntries => &[],
        }
    }

    pub(super) fn legacy_len(&self, project: &Project) -> usize {
        match self {
            Self::AllEntries => project.entries().len(),
            Self::Indices(indices) => indices.len(),
            Self::AllChildren => 0,
        }
    }

    pub(super) fn legacy_index(&self, project: &Project, row: usize) -> Option<usize> {
        match self {
            Self::AllEntries => (row < project.entries().len()).then_some(row),
            Self::Indices(indices) => indices.get(row).copied(),
            Self::AllChildren => None,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum QueryMode {
    Legacy,
    Reference,
}

struct QueryKey {
    snapshot_id: ProjectSnapshotId,
    mode: QueryMode,
    directory: Option<PathBuf>,
    query: String,
}

impl QueryKey {
    fn matches(
        &self,
        snapshot_id: &ProjectSnapshotId,
        mode: QueryMode,
        directory: Option<&Path>,
        query: &str,
    ) -> bool {
        &self.snapshot_id == snapshot_id
            && self.mode == mode
            && self.directory.as_deref() == directory
            && self.query == query
    }

    fn new(
        snapshot_id: ProjectSnapshotId,
        mode: QueryMode,
        directory: Option<&Path>,
        query: &str,
    ) -> Self {
        Self {
            snapshot_id,
            mode,
            directory: directory.map(|path| path.to_path_buf()),
            query: query.to_owned(),
        }
    }
}

struct QueryResult {
    key: QueryKey,
    indices: CachedEntryIndices,
}

#[derive(Default)]
pub(super) struct ProjectBrowserCache {
    result: Option<QueryResult>,
}

impl ProjectBrowserCache {
    pub(super) fn reference_indices(
        &mut self,
        project: &Project,
        directory: &Path,
        query: &str,
    ) -> CachedEntryIndices {
        let snapshot_id = project.snapshot_id();

        if let Some(result) = &self.result
            && result
                .key
                .matches(&snapshot_id, QueryMode::Reference, Some(directory), query)
        {
            return result.indices.clone();
        }

        let indices = if query.is_empty() {
            CachedEntryIndices::AllChildren
        } else {
            Self::filter_children(project, directory, query)
        };

        self.replace_result(
            snapshot_id,
            QueryMode::Reference,
            Some(directory),
            query,
            indices,
        )
    }

    pub(super) fn legacy_indices(&mut self, project: &Project, query: &str) -> CachedEntryIndices {
        let snapshot_id = project.snapshot_id();

        if let Some(result) = &self.result
            && result
                .key
                .matches(&snapshot_id, QueryMode::Legacy, None, query)
        {
            return result.indices.clone();
        }

        let indices = if query.is_empty() {
            CachedEntryIndices::AllEntries
        } else {
            Self::ranked_matches(project, query)
        };

        self.replace_result(snapshot_id, QueryMode::Legacy, None, query, indices)
    }

    pub(super) fn clear(&mut self) {
        self.result = None;
    }

    fn replace_result(
        &mut self,
        snapshot_id: ProjectSnapshotId,
        mode: QueryMode,
        directory: Option<&Path>,
        query: &str,
        indices: CachedEntryIndices,
    ) -> CachedEntryIndices {
        self.result = Some(QueryResult {
            key: QueryKey::new(snapshot_id, mode, directory, query),
            indices: indices.clone(),
        });
        indices
    }

    fn filter_children(project: &Project, directory: &Path, query: &str) -> CachedEntryIndices {
        let query = query.to_lowercase();
        let Some(children) = project.children(directory) else {
            return CachedEntryIndices::Indices(Arc::from(Vec::<usize>::new()));
        };

        let entries = project.entries();
        let indices = children
            .iter()
            .filter_map(|&index| {
                let entry = entries.get(index)?;
                let name = entry
                    .relative_path
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_lowercase();
                name.contains(&query).then_some(index)
            })
            .collect::<Vec<_>>();

        CachedEntryIndices::Indices(indices.into())
    }

    fn ranked_matches(project: &Project, query: &str) -> CachedEntryIndices {
        let labels: Vec<Cow<'_, str>> = project
            .entries()
            .iter()
            .map(|entry| entry.relative_path.to_string_lossy())
            .collect();
        let indices = ranked_indices(query, labels.iter().map(|label| label.as_ref()));
        CachedEntryIndices::Indices(indices.into())
    }
}
