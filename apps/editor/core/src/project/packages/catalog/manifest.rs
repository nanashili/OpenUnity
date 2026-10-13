//! Bounded manifest parsing and generic unresolved-dependency diagnostics.

use std::path::{Path, PathBuf};

use super::super::filesystem::{project_manifest_path, read_bounded_file};
use super::super::metadata::project_manifest_dependencies;
use super::super::model::{CatalogDiagnostics, PackageSourceDescriptor};
use crate::project::ProjectPackageDiagnosticKind;

const MAX_PROJECT_MANIFEST_BYTES: usize = 256 * 1024;

pub(super) fn read_manifest_dependencies(
    project_root: &Path,
    packages_root: &Path,
    diagnostics: &mut CatalogDiagnostics,
) -> Option<Vec<String>> {
    let path = project_manifest_path(project_root, packages_root, diagnostics)?;
    let bytes = match read_bounded_file(&path, MAX_PROJECT_MANIFEST_BYTES, None) {
        Ok(bytes) => bytes,
        Err(_) => {
            diagnostics.push(
                ProjectPackageDiagnosticKind::ManifestInvalid,
                None,
                Some(PathBuf::from("manifest.json")),
            );
            return None;
        }
    };

    project_manifest_dependencies(&bytes, diagnostics)
}

pub(super) fn report_unmatched_dependencies(
    sources: &[PackageSourceDescriptor],
    dependencies: Option<Vec<String>>,
    diagnostics: &mut CatalogDiagnostics,
) {
    let Some(dependencies) = dependencies else {
        return;
    };

    for dependency in dependencies {
        let is_embedded = sources
            .binary_search_by(|source| source.name.cmp(&dependency))
            .is_ok();
        if !is_embedded {
            diagnostics.push(
                ProjectPackageDiagnosticKind::DependencyUnresolved,
                None,
                None,
            );
        }
    }
}
