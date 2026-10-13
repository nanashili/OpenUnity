//! Borrowed views that keep source-relative paths paired with their index.

use std::path::Path;

use super::super::index::ProjectIndex;
use super::super::{ProjectDiagnostic, ProjectEntry, ProjectPackageDiagnostic};
use super::ProjectSourceKey;

/// Diagnostics associated with the selected source view.
#[derive(Clone, Copy, Debug)]
pub enum ProjectSourceDiagnostics<'a> {
    /// Existing Assets traversal diagnostics.
    Assets(&'a [ProjectDiagnostic]),
    /// Bounded package-content traversal diagnostics.
    EmbeddedPackage(&'a [ProjectPackageDiagnostic]),
}

impl ProjectSourceDiagnostics<'_> {
    /// Whether the selected source has no content-discovery diagnostics.
    pub fn is_empty(&self) -> bool {
        match self {
            Self::Assets(diagnostics) => diagnostics.is_empty(),
            Self::EmbeddedPackage(diagnostics) => diagnostics.is_empty(),
        }
    }
}

/// Borrowed access to one source's root, entries, lookup index, and diagnostics.
#[derive(Clone, Copy, Debug)]
pub struct ProjectSource<'a> {
    key: &'a ProjectSourceKey,
    root: &'a Path,
    entries: &'a [ProjectEntry],
    index: &'a ProjectIndex,
    diagnostics: ProjectSourceDiagnostics<'a>,
}

impl<'a> ProjectSource<'a> {
    pub(super) fn new(
        key: &'a ProjectSourceKey,
        root: &'a Path,
        entries: &'a [ProjectEntry],
        index: &'a ProjectIndex,
        diagnostics: ProjectSourceDiagnostics<'a>,
    ) -> Self {
        Self {
            key,
            root,
            entries,
            index,
            diagnostics,
        }
    }

    /// Stable identity that qualifies the relative paths in this view.
    pub fn key(&self) -> &ProjectSourceKey {
        self.key
    }

    /// Canonical source root.
    pub fn root(&self) -> &Path {
        self.root
    }

    /// Deterministically sorted entries relative to this source root.
    pub fn entries(&self) -> &[ProjectEntry] {
        self.entries
    }

    /// Finds one exact source-relative entry path.
    pub fn entry(&self, relative_path: &Path) -> Option<&'a ProjectEntry> {
        self.entries
            .binary_search_by(|entry| entry.relative_path.as_path().cmp(relative_path))
            .ok()
            .map(|index| &self.entries[index])
    }

    /// Returns immediate child indices for a source-relative directory.
    /// The empty path denotes this source root; indices refer to `entries()`.
    pub fn children(&self, directory: &Path) -> Option<&'a [usize]> {
        self.index.children(self.entries, directory)
    }

    /// Returns indices of discovered directories in `entries()` order.
    pub fn directories(&self) -> &[usize] {
        self.index.directories()
    }

    /// Reports whether a known directory in this source has immediate children.
    pub fn has_children(&self, directory: &Path) -> bool {
        self.children(directory)
            .is_some_and(|children| !children.is_empty())
    }

    /// Returns content-discovery diagnostics for this source only.
    pub fn diagnostics(&self) -> ProjectSourceDiagnostics<'a> {
        self.diagnostics
    }
}
