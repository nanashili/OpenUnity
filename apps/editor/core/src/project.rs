//! Read-only discovery of a source project; no import or Unity compatibility is implied.

use std::path::PathBuf;

mod discovery;
mod model;
mod opening;

pub use model::{
    MAX_PROJECT_DEPTH, MAX_PROJECT_ENTRIES, MAX_PROJECT_VERSION_BYTES, ProjectDiagnostic,
    ProjectDiagnosticKind, ProjectEntry, ProjectEntryKind, ProjectError,
};

#[derive(Debug)]
/// A read-only snapshot of project metadata and Assets entry paths.
pub struct Project {
    root: PathBuf,
    editor_version: String,
    entries: Vec<ProjectEntry>,
    diagnostics: Vec<ProjectDiagnostic>,
}

#[cfg(test)]
mod tests;
