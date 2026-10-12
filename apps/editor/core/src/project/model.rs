//! Project entry records, discovery bounds and opening errors.

use std::error::Error;
use std::fmt;
use std::path::PathBuf;

/// Maximum inspected entries, including rejected entries.
pub const MAX_PROJECT_ENTRIES: usize = 100_000;
/// Maximum Assets traversal depth.
pub const MAX_PROJECT_DEPTH: usize = 256;
/// Maximum ProjectVersion.txt input size.
pub const MAX_PROJECT_VERSION_BYTES: usize = 64 * 1024;

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
/// Supported filesystem entry kinds; contents are never imported.
pub enum ProjectEntryKind {
    /// A real directory.
    Directory,
    /// A regular file.
    File,
}

#[derive(Clone, Debug, Eq, PartialEq)]
/// One discovered Assets-relative entry.
pub struct ProjectEntry {
    /// Relative to Assets, never an absolute path or a parent traversal.
    pub relative_path: PathBuf,
    /// Filesystem kind.
    pub kind: ProjectEntryKind,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
/// Why an entry could not be included.
pub enum ProjectDiagnosticKind {
    /// Symbolic links are not followed.
    Symlink,
    /// The resolved path escaped the project root.
    OutsideRoot,
    /// Filesystem access failed.
    Inaccessible,
    /// The entry is neither a directory nor a regular file.
    UnsupportedEntry,
    /// Traversal reached its depth bound.
    DepthLimit,
    /// Traversal reached its entry bound.
    EntryLimit,
}

#[derive(Clone, Debug, Eq, PartialEq)]
/// A nonfatal discovery failure.
pub struct ProjectDiagnostic {
    /// Relative to Assets; the empty path identifies Assets itself.
    pub relative_path: PathBuf,
    /// Failure classification.
    pub kind: ProjectDiagnosticKind,
    /// User-facing explanation.
    pub message: String,
}

#[derive(Debug)]
/// Failure to open a required project path or version header.
pub struct ProjectError {
    /// Path involved in the operation.
    pub path: PathBuf,
    /// User-facing explanation.
    pub message: String,
}

impl fmt::Display for ProjectError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Cannot open project at {}: {}",
            self.path.display(),
            self.message
        )
    }
}

impl Error for ProjectError {}
