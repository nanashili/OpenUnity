//! Admission of direct embedded-package directories and their metadata files.

use std::fs::{self, DirEntry};
use std::path::{Path, PathBuf};

use super::super::model::CatalogDiagnostics;
use crate::project::ProjectPackageDiagnosticKind;
use crate::project::packages::is_valid_technical_name;

pub(in crate::project::packages) struct PackageDirectory {
    pub(in crate::project::packages) name: String,
    pub(in crate::project::packages) root: PathBuf,
}

pub(in crate::project::packages) fn classify_package_directory(
    child: &DirEntry,
    name: &str,
    packages_root: &Path,
    project_root: &Path,
    diagnostics: &mut CatalogDiagnostics,
) -> Option<PackageDirectory> {
    let path = child.path();
    let metadata = match fs::symlink_metadata(&path) {
        Ok(metadata) => metadata,
        Err(_) => {
            diagnose_candidate(diagnostics, name);
            return None;
        }
    };

    if metadata.file_type().is_symlink() {
        diagnostics.push(
            ProjectPackageDiagnosticKind::Symlink,
            Some(name.to_owned()),
            None,
        );
        return None;
    }

    if metadata.is_file() {
        return None;
    }

    if !metadata.is_dir() {
        diagnostics.push(
            ProjectPackageDiagnosticKind::UnsupportedEntry,
            Some(name.to_owned()),
            None,
        );
        return None;
    }

    if !is_valid_technical_name(name) {
        diagnostics.push(ProjectPackageDiagnosticKind::CandidateInvalid, None, None);
        return None;
    }

    let canonical = match fs::canonicalize(path) {
        Ok(path) => path,
        Err(_) => {
            diagnose_candidate(diagnostics, name);
            return None;
        }
    };

    if !canonical.starts_with(packages_root) || !canonical.starts_with(project_root) {
        diagnostics.push(
            ProjectPackageDiagnosticKind::OutsideSource,
            Some(name.to_owned()),
            None,
        );
        return None;
    }

    Some(PackageDirectory {
        name: name.to_owned(),
        root: canonical,
    })
}

pub(in crate::project::packages) fn package_json_path(
    package: &PackageDirectory,
    packages_root: &Path,
    project_root: &Path,
    diagnostics: &mut CatalogDiagnostics,
) -> Option<PathBuf> {
    let path = package.root.join("package.json");
    let relative_path = PathBuf::from("package.json");
    let metadata = match fs::symlink_metadata(&path) {
        Ok(metadata) => metadata,
        Err(_) => {
            diagnose_package_metadata(
                diagnostics,
                package,
                ProjectPackageDiagnosticKind::CandidateInvalid,
            );
            return None;
        }
    };

    if metadata.file_type().is_symlink() {
        diagnostics.push(
            ProjectPackageDiagnosticKind::Symlink,
            Some(package.name.clone()),
            Some(relative_path),
        );
        return None;
    }

    if !metadata.is_file() {
        diagnose_package_metadata(
            diagnostics,
            package,
            ProjectPackageDiagnosticKind::CandidateInvalid,
        );
        return None;
    }

    let canonical = match fs::canonicalize(&path) {
        Ok(path) => path,
        Err(_) => {
            diagnose_package_metadata(
                diagnostics,
                package,
                ProjectPackageDiagnosticKind::MetadataReadFailed,
            );
            return None;
        }
    };

    if !canonical.starts_with(&package.root)
        || !canonical.starts_with(packages_root)
        || !canonical.starts_with(project_root)
    {
        diagnostics.push(
            ProjectPackageDiagnosticKind::OutsideSource,
            Some(package.name.clone()),
            Some(relative_path),
        );
        return None;
    }

    Some(canonical)
}

fn diagnose_candidate(diagnostics: &mut CatalogDiagnostics, name: &str) {
    diagnostics.push(
        ProjectPackageDiagnosticKind::CandidateInvalid,
        Some(name.to_owned()),
        None,
    );
}

fn diagnose_package_metadata(
    diagnostics: &mut CatalogDiagnostics,
    package: &PackageDirectory,
    kind: ProjectPackageDiagnosticKind,
) {
    diagnostics.push(
        kind,
        Some(package.name.clone()),
        Some(PathBuf::from("package.json")),
    );
}
