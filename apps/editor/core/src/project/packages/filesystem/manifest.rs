//! Admission of the optional project package manifest.

use std::fs;
use std::path::{Path, PathBuf};

use super::super::model::CatalogDiagnostics;
use crate::project::ProjectPackageDiagnosticKind;

pub(in crate::project::packages) fn project_manifest_path(
    project_root: &Path,
    packages_root: &Path,
    diagnostics: &mut CatalogDiagnostics,
) -> Option<PathBuf> {
    let path = packages_root.join("manifest.json");
    let relative_path = PathBuf::from("manifest.json");
    let metadata = match fs::symlink_metadata(&path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return None,
        Err(_) => {
            diagnose_manifest(diagnostics, relative_path);
            return None;
        }
    };

    if metadata.file_type().is_symlink() || !metadata.is_file() {
        diagnose_manifest(diagnostics, relative_path);
        return None;
    }

    let canonical = match fs::canonicalize(path) {
        Ok(path) => path,
        Err(_) => {
            diagnose_manifest(diagnostics, relative_path);
            return None;
        }
    };

    if !canonical.starts_with(packages_root) || !canonical.starts_with(project_root) {
        diagnose_manifest(diagnostics, relative_path);
        return None;
    }

    Some(canonical)
}

fn diagnose_manifest(diagnostics: &mut CatalogDiagnostics, relative_path: PathBuf) {
    diagnostics.push(
        ProjectPackageDiagnosticKind::ManifestInvalid,
        None,
        Some(relative_path),
    );
}
