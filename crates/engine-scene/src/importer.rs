use crate::{Result, ScenePlan};
use std::path::Path;

mod bindings;
mod document_values;
mod documents;
mod filesystem;
mod objects;
mod render_reference_import;
mod scripts;

pub(crate) use document_values::{flag, float, vector};

/// Read actual scene/meta files and an explicit type/field binding manifest.
/// Paths must be within an absolute nonsymlink project root. No file is modified.
/// Only the reviewed authored grammar and exact skipped settings are supported.
pub fn import_project(root: &Path, scene: &Path, bindings: &Path) -> Result<ScenePlan> {
    render_reference_import::import_project_legacy(root, scene, bindings)
}

pub use render_reference_import::{ProjectRenderImport, import_project_with_render_references};
