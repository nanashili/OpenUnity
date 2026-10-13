//! Per-snapshot query results shared by the virtualized Project browser views.

mod filters;
mod indices;

pub(super) use indices::CachedEntryIndices;

use std::path::{Path, PathBuf};

use openunity_editor_core::project::{Project, ProjectSnapshotId, ProjectSource, ProjectSourceKey};

#[derive(Clone, Copy, PartialEq, Eq)]
enum QueryMode {
    Legacy,
    Reference,
}

struct QueryKey {
    snapshot_id: ProjectSnapshotId,
    source_key: ProjectSourceKey,
    mode: QueryMode,
    directory: Option<PathBuf>,
    query: String,
}

impl QueryKey {
    fn matches(
        &self,
        snapshot_id: &ProjectSnapshotId,
        source_key: &ProjectSourceKey,
        mode: QueryMode,
        directory: Option<&Path>,
        query: &str,
    ) -> bool {
        &self.snapshot_id == snapshot_id
            && &self.source_key == source_key
            && self.mode == mode
            && self.directory.as_deref() == directory
            && self.query == query
    }

    fn new(
        snapshot_id: ProjectSnapshotId,
        source_key: &ProjectSourceKey,
        mode: QueryMode,
        directory: Option<&Path>,
        query: &str,
    ) -> Self {
        Self {
            snapshot_id,
            source_key: source_key.clone(),
            mode,
            directory: directory.map(Path::to_path_buf),
            query: query.to_owned(),
        }
    }
}

struct QueryResult {
    key: QueryKey,
    indices: CachedEntryIndices,
    total_entries: usize,
}

#[derive(Default)]
pub(super) struct ProjectBrowserCache {
    result: Option<QueryResult>,
}

impl ProjectBrowserCache {
    pub(super) fn reference_indices(
        &mut self,
        project: &Project,
        source: ProjectSource<'_>,
        directory: &Path,
        query: &str,
    ) -> CachedEntryIndices {
        let snapshot_id = project.snapshot_id();
        let source_key = source.key();

        if let Some(result) = &self.result
            && result.key.matches(
                &snapshot_id,
                source_key,
                QueryMode::Reference,
                Some(directory),
                query,
            )
        {
            return result.indices.clone();
        }

        let (indices, total_entries) = if query.is_empty() {
            filters::all_visible_children(source, directory)
        } else {
            filters::filter_children(source, directory, query)
        };

        let key = QueryKey::new(
            snapshot_id,
            source_key,
            QueryMode::Reference,
            Some(directory),
            query,
        );

        self.replace_result(key, indices, total_entries).0
    }

    pub(super) fn legacy_indices(
        &mut self,
        project: &Project,
        source: ProjectSource<'_>,
        query: &str,
    ) -> (CachedEntryIndices, usize) {
        let snapshot_id = project.snapshot_id();
        let source_key = source.key();

        if let Some(result) = &self.result
            && result
                .key
                .matches(&snapshot_id, source_key, QueryMode::Legacy, None, query)
        {
            return (result.indices.clone(), result.total_entries);
        }

        let (indices, total_entries) = if query.is_empty() {
            filters::all_visible_entries(source)
        } else {
            filters::ranked_matches(source, query)
        };

        let key = QueryKey::new(snapshot_id, source_key, QueryMode::Legacy, None, query);

        self.replace_result(key, indices, total_entries)
    }

    pub(super) fn clear(&mut self) {
        self.result = None;
    }

    fn replace_result(
        &mut self,
        key: QueryKey,
        indices: CachedEntryIndices,
        total_entries: usize,
    ) -> (CachedEntryIndices, usize) {
        self.result = Some(QueryResult {
            key,
            indices: indices.clone(),
            total_entries,
        });
        (indices, total_entries)
    }
}
