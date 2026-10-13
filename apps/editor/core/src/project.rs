//! Read-only discovery of a source project; no import or Unity compatibility is implied.

use std::path::PathBuf;

mod discovery;
mod index;
mod model;
mod opening;
mod package_diagnostic;
mod packages;
mod source;

use index::ProjectIndex;

pub use index::ProjectSnapshotId;
pub use model::{
    MAX_PROJECT_DEPTH, MAX_PROJECT_ENTRIES, MAX_PROJECT_VERSION_BYTES, ProjectDiagnostic,
    ProjectDiagnosticKind, ProjectEntry, ProjectEntryKind, ProjectError,
};
pub use package_diagnostic::{ProjectPackageDiagnostic, ProjectPackageDiagnosticKind};
pub use source::{ProjectPackage, ProjectSource, ProjectSourceDiagnostics, ProjectSourceKey};

#[derive(Debug)]
/// A read-only snapshot of project metadata and source-relative entry indexes.
pub struct Project {
    root: PathBuf,
    editor_version: String,
    entries: Vec<ProjectEntry>,
    diagnostics: Vec<ProjectDiagnostic>,
    index: ProjectIndex,
    snapshot_id: ProjectSnapshotId,
    assets_root: PathBuf,
    packages: Vec<ProjectPackage>,
    package_diagnostics: Vec<ProjectPackageDiagnostic>,
}

#[cfg(test)]
mod tests;
