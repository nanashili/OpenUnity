//! Compact query results over one source's immutable entry index.

use std::borrow::Cow;
use std::path::Path;
use std::sync::Arc;

use openunity_editor_core::project::ProjectSource;

#[derive(Clone)]
pub(in crate::panels::project) enum CachedEntryIndices {
    AllEntries,
    AllChildren,
    Indices(Arc<[usize]>),
}

impl CachedEntryIndices {
    pub(in crate::panels::project) fn reference_slice<'a>(
        &'a self,
        source: ProjectSource<'a>,
        directory: &Path,
    ) -> Cow<'a, [usize]> {
        match self {
            Self::AllChildren | Self::AllEntries => source
                .children(directory)
                .map_or(Cow::Borrowed(&[]), Cow::Borrowed),
            Self::Indices(indices) => Cow::Borrowed(indices),
        }
    }

    pub(in crate::panels::project) fn legacy_len(&self, source: ProjectSource<'_>) -> usize {
        match self {
            Self::AllEntries => source.entries().len(),
            Self::Indices(indices) => indices.len(),
            Self::AllChildren => 0,
        }
    }

    pub(in crate::panels::project) fn legacy_index(
        &self,
        source: ProjectSource<'_>,
        row: usize,
    ) -> Option<usize> {
        match self {
            Self::AllEntries => (row < source.entries().len()).then_some(row),
            Self::Indices(indices) => indices.get(row).copied(),
            Self::AllChildren => None,
        }
    }
}
