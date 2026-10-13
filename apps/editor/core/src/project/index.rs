//! Immutable entry-position index and identity for one opened Project snapshot.

use std::path::Path;
use std::sync::Arc;

use super::{Project, ProjectEntry, ProjectEntryKind};

#[derive(Debug)]
pub(super) struct ProjectIndex {
    directories: Box<[usize]>,
    child_offsets: Box<[usize]>,
    child_entries: Box<[usize]>,
}

impl ProjectIndex {
    /// Builds directory groups without copying paths or project entries.
    ///
    /// The error is the entry index whose parent is absent from the snapshot.
    pub(super) fn build(entries: &[ProjectEntry]) -> Result<Self, usize> {
        let directories = directory_indices(entries);
        let group_count = directories.len() + 1;
        let mut child_counts = vec![0; group_count];

        for entry_index in 0..entries.len() {
            let group = child_group(entries, &directories, entry_index)?;
            child_counts[group] += 1;
        }

        let child_offsets = offsets_from_counts(&child_counts);
        let mut insertion_positions = child_offsets[..group_count].to_vec();
        let mut child_entries = vec![0; entries.len()];

        for entry_index in 0..entries.len() {
            let group = child_group(entries, &directories, entry_index)?;
            let position = &mut insertion_positions[group];
            child_entries[*position] = entry_index;
            *position += 1;
        }

        Ok(Self {
            directories: directories.into_boxed_slice(),
            child_offsets: child_offsets.into_boxed_slice(),
            child_entries: child_entries.into_boxed_slice(),
        })
    }

    pub(super) fn directories(&self) -> &[usize] {
        &self.directories
    }

    pub(super) fn children<'a>(
        &'a self,
        entries: &[ProjectEntry],
        directory: &Path,
    ) -> Option<&'a [usize]> {
        let group = self.directory_group(entries, directory)?;
        let start = self.child_offsets[group];
        let end = self.child_offsets[group + 1];
        self.child_entries.get(start..end)
    }

    fn directory_group(&self, entries: &[ProjectEntry], path: &Path) -> Option<usize> {
        if path.as_os_str().is_empty() {
            return Some(self.directories.len());
        }

        self.directories
            .binary_search_by(|&entry_index| entries[entry_index].relative_path.as_path().cmp(path))
            .ok()
    }
}

fn directory_indices(entries: &[ProjectEntry]) -> Vec<usize> {
    entries
        .iter()
        .enumerate()
        .filter_map(|(index, entry)| (entry.kind == ProjectEntryKind::Directory).then_some(index))
        .collect()
}

fn child_group(
    entries: &[ProjectEntry],
    directories: &[usize],
    entry_index: usize,
) -> Result<usize, usize> {
    let entry = &entries[entry_index];
    let parent = entry
        .relative_path
        .parent()
        .unwrap_or_else(|| Path::new(""));

    if parent.as_os_str().is_empty() {
        return Ok(directories.len());
    }

    directories
        .binary_search_by(|&directory_index| {
            entries[directory_index].relative_path.as_path().cmp(parent)
        })
        .map_err(|_| entry_index)
}

fn offsets_from_counts(counts: &[usize]) -> Vec<usize> {
    let mut offsets = Vec::with_capacity(counts.len() + 1);
    offsets.push(0);

    for count in counts {
        let previous = offsets[offsets.len() - 1];
        offsets.push(previous + count);
    }

    offsets
}

/// Strong identity for caches derived from one immutable Project snapshot.
#[derive(Clone, Debug)]
pub struct ProjectSnapshotId(Arc<SnapshotIdentity>);

#[derive(Debug)]
struct SnapshotIdentity;

impl ProjectSnapshotId {
    pub(super) fn new() -> Self {
        Self(Arc::new(SnapshotIdentity))
    }
}

impl PartialEq for ProjectSnapshotId {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}

impl Eq for ProjectSnapshotId {}

impl Project {
    /// Finds one exact Assets-relative entry path.
    pub fn entry(&self, relative_path: &Path) -> Option<&ProjectEntry> {
        self.entries
            .binary_search_by(|entry| entry.relative_path.as_path().cmp(relative_path))
            .ok()
            .map(|index| &self.entries[index])
    }

    /// Returns immediate child indices for an Assets-relative directory.
    /// The empty path denotes the Assets root; indices refer to `entries()`.
    pub fn children(&self, directory: &Path) -> Option<&[usize]> {
        self.index.children(&self.entries, directory)
    }

    /// Returns indices of discovered directories in `entries()` order.
    pub fn directories(&self) -> &[usize] {
        self.index.directories()
    }

    /// Reports whether a known Assets-relative directory has immediate children.
    pub fn has_children(&self, directory: &Path) -> bool {
        self.children(directory)
            .is_some_and(|children| !children.is_empty())
    }

    /// Returns the strong identity of this opened Project snapshot.
    pub fn snapshot_id(&self) -> ProjectSnapshotId {
        self.snapshot_id.clone()
    }
}
