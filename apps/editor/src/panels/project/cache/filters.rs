//! Source-local search and display filtering for cached entry queries.

use std::borrow::Cow;
use std::path::Path;
use std::sync::Arc;

use super::indices::CachedEntryIndices;
use crate::panels::project::visibility;
use openunity_editor_core::project::ProjectSource;
use openunity_editor_core::search::ranked_indices;

pub(super) fn all_visible_entries(source: ProjectSource<'_>) -> (CachedEntryIndices, usize) {
    let entries = source.entries();
    let indices = if entries.iter().all(visibility::entry_is_visible) {
        CachedEntryIndices::AllEntries
    } else {
        CachedEntryIndices::Indices(visibility::visible_indices(entries).into())
    };
    let count = match &indices {
        CachedEntryIndices::AllEntries => entries.len(),
        CachedEntryIndices::Indices(indices) => indices.len(),
        CachedEntryIndices::AllChildren => 0,
    };

    (indices, count)
}

pub(super) fn all_visible_children(
    source: ProjectSource<'_>,
    directory: &Path,
) -> (CachedEntryIndices, usize) {
    let Some(children) = source.children(directory) else {
        return (CachedEntryIndices::Indices(Arc::from([])), 0);
    };

    let entries = source.entries();
    let has_hidden_metadata = children.iter().any(|&index| {
        entries
            .get(index)
            .is_some_and(|entry| !visibility::entry_is_visible(entry))
    });
    if !has_hidden_metadata {
        return (CachedEntryIndices::AllChildren, children.len());
    }

    let indices = children
        .iter()
        .filter(|&&index| entries.get(index).is_some_and(visibility::entry_is_visible))
        .copied()
        .collect::<Vec<_>>();
    let count = indices.len();
    (CachedEntryIndices::Indices(indices.into()), count)
}

pub(super) fn filter_children(
    source: ProjectSource<'_>,
    directory: &Path,
    query: &str,
) -> (CachedEntryIndices, usize) {
    let query = query.to_lowercase();
    let Some(children) = source.children(directory) else {
        return (CachedEntryIndices::Indices(Arc::from([])), 0);
    };

    let entries = source.entries();
    let indices = children
        .iter()
        .filter_map(|&index| {
            let entry = entries.get(index)?;
            if !visibility::entry_is_visible(entry) {
                return None;
            }

            let name = entry
                .relative_path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_lowercase();
            name.contains(&query).then_some(index)
        })
        .collect::<Vec<_>>();
    let count = indices.len();

    (CachedEntryIndices::Indices(indices.into()), count)
}

pub(super) fn ranked_matches(
    source: ProjectSource<'_>,
    query: &str,
) -> (CachedEntryIndices, usize) {
    let entries = source.entries();
    let has_hidden_metadata = entries
        .iter()
        .any(|entry| !visibility::entry_is_visible(entry));
    let candidate_indices = has_hidden_metadata.then(|| visibility::visible_indices(entries));
    let labels: Vec<Cow<'_, str>> = match &candidate_indices {
        Some(indices) => indices
            .iter()
            .filter_map(|&index| entries.get(index))
            .map(|entry| entry.relative_path.to_string_lossy())
            .collect(),
        None => entries
            .iter()
            .map(|entry| entry.relative_path.to_string_lossy())
            .collect(),
    };
    let count = candidate_indices.as_ref().map_or(entries.len(), Vec::len);
    let ranked_positions = ranked_indices(query, labels.iter().map(|label| label.as_ref()));

    let indices = match candidate_indices {
        Some(candidates) => ranked_positions
            .into_iter()
            .filter_map(|position| candidates.get(position).copied())
            .collect::<Vec<_>>(),
        None => ranked_positions,
    };

    (CachedEntryIndices::Indices(indices.into()), count)
}
