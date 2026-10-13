//! Admission of the project's direct Packages root.

use std::fs;
use std::path::{Path, PathBuf};

use super::super::model::CatalogDiagnostics;
use crate::project::ProjectPackageDiagnosticKind;

pub(in crate::project::packages) enum PackagesRoot {
    Missing,
    Available(PathBuf),
    Invalid,
    Inaccessible,
}

pub(in crate::project::packages) fn inspect_packages_root(
    project_root: &Path,
    diagnostics: &mut CatalogDiagnostics,
) -> PackagesRoot {
    let path = project_root.join("Packages");
    let metadata = match fs::symlink_metadata(&path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return PackagesRoot::Missing;
        }
        Err(_) => {
            diagnostics.push(
                ProjectPackageDiagnosticKind::PackagesRootInaccessible,
                None,
                None,
            );
            return PackagesRoot::Inaccessible;
        }
    };

    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        diagnostics.push(
            ProjectPackageDiagnosticKind::PackagesRootInvalid,
            None,
            None,
        );
        return PackagesRoot::Invalid;
    }

    let canonical = match fs::canonicalize(&path) {
        Ok(path) => path,
        Err(_) => {
            diagnostics.push(
                ProjectPackageDiagnosticKind::PackagesRootInaccessible,
                None,
                None,
            );
            return PackagesRoot::Inaccessible;
        }
    };

    if !canonical.starts_with(project_root) {
        diagnostics.push(
            ProjectPackageDiagnosticKind::PackagesRootInvalid,
            None,
            None,
        );
        return PackagesRoot::Invalid;
    }

    PackagesRoot::Available(canonical)
}
