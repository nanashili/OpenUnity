//! Bounded package metadata inspection and catalog assembly.

use std::path::{Path, PathBuf};

use super::super::filesystem::{
    BoundedReadError, PackageDirectory, PackagesRoot, classify_package_directory,
    inspect_packages_root, package_json_path, read_bounded_file,
};
use super::super::listing::{ChildListing, collect_direct_children};
use super::super::metadata::parse_package_manifest;
use super::super::model::{CatalogDiagnostics, PackageCatalog, PackageSourceDescriptor};
use super::candidates::{NamedChild, sort_named_children};
use super::manifest::{read_manifest_dependencies, report_unmatched_dependencies};
use crate::project::ProjectPackageDiagnosticKind;

const MAX_PACKAGE_JSON_BYTES: usize = 64 * 1024;
const MAX_TOTAL_PACKAGE_METADATA_BYTES: usize = 64 * 1024 * 1024;
const MAX_RETAINED_PACKAGES: usize = 128;

pub(in crate::project) fn discover_embedded_packages(project_root: &Path) -> PackageCatalog {
    let mut diagnostics = CatalogDiagnostics::new();
    let mut sources = Vec::new();
    let mut catalog_incomplete = false;

    let packages_root = match inspect_packages_root(project_root, &mut diagnostics) {
        PackagesRoot::Available(path) => path,
        PackagesRoot::Missing => return finish_catalog(sources, diagnostics),
        PackagesRoot::Invalid | PackagesRoot::Inaccessible => {
            return finish_catalog(sources, diagnostics);
        }
    };

    let candidates = match collect_candidates(&packages_root, &mut diagnostics) {
        Some(candidates) => candidates,
        None => {
            catalog_incomplete = true;
            Vec::new()
        }
    };

    let mut metadata_bytes_read = 0;
    inspect_candidates(
        project_root,
        &packages_root,
        &candidates,
        &mut metadata_bytes_read,
        &mut sources,
        &mut diagnostics,
        &mut catalog_incomplete,
    );

    let dependencies = read_manifest_dependencies(project_root, &packages_root, &mut diagnostics);
    if !catalog_incomplete {
        report_unmatched_dependencies(&sources, dependencies, &mut diagnostics);
    }

    finish_catalog(sources, diagnostics)
}

fn collect_candidates(
    packages_root: &Path,
    diagnostics: &mut CatalogDiagnostics,
) -> Option<Vec<NamedChild>> {
    match collect_direct_children(packages_root) {
        ChildListing::Complete(children) => Some(sort_named_children(children, diagnostics)),
        ChildListing::Overflow => {
            diagnostics.push(ProjectPackageDiagnosticKind::CatalogOverflow, None, None);
            None
        }
        ChildListing::Failed => {
            diagnostics.push(
                ProjectPackageDiagnosticKind::PackagesRootInaccessible,
                None,
                None,
            );
            None
        }
    }
}

fn inspect_candidates(
    project_root: &Path,
    packages_root: &Path,
    candidates: &[NamedChild],
    metadata_bytes_read: &mut usize,
    sources: &mut Vec<PackageSourceDescriptor>,
    diagnostics: &mut CatalogDiagnostics,
    catalog_incomplete: &mut bool,
) {
    for (candidate_index, candidate) in candidates.iter().enumerate() {
        if sources.len() == MAX_RETAINED_PACKAGES {
            if candidate_index < candidates.len() {
                diagnostics.push(ProjectPackageDiagnosticKind::CatalogTruncated, None, None);
                *catalog_incomplete = true;
            }
            return;
        }

        let Some(package) = classify_package_directory(
            &candidate.entry,
            &candidate.name,
            packages_root,
            project_root,
            diagnostics,
        ) else {
            continue;
        };

        let Some(metadata_path) =
            package_json_path(&package, packages_root, project_root, diagnostics)
        else {
            continue;
        };

        let Some(metadata) = read_package_metadata(
            &package,
            &metadata_path,
            metadata_bytes_read,
            diagnostics,
            catalog_incomplete,
        ) else {
            if *catalog_incomplete {
                return;
            }
            continue;
        };

        if let Some(descriptor) =
            parse_package_manifest(&metadata, &package.name, package.root, diagnostics)
        {
            sources.push(descriptor);
        }
    }
}

fn read_package_metadata(
    package: &PackageDirectory,
    metadata_path: &Path,
    metadata_bytes_read: &mut usize,
    diagnostics: &mut CatalogDiagnostics,
    catalog_incomplete: &mut bool,
) -> Option<Vec<u8>> {
    let aggregate_remaining = MAX_TOTAL_PACKAGE_METADATA_BYTES.saturating_sub(*metadata_bytes_read);
    let result = read_bounded_file(
        metadata_path,
        MAX_PACKAGE_JSON_BYTES,
        Some(aggregate_remaining),
    );
    *metadata_bytes_read = metadata_bytes_read.saturating_add(bytes_read(&result));

    match result {
        Ok(bytes) => Some(bytes),
        Err(BoundedReadError::Failed { .. }) => {
            diagnostics.push(
                ProjectPackageDiagnosticKind::MetadataReadFailed,
                Some(package.name.clone()),
                Some(PathBuf::from("package.json")),
            );
            None
        }
        Err(BoundedReadError::LimitExceeded { .. }) => {
            diagnostics.push(
                ProjectPackageDiagnosticKind::MetadataLimit,
                Some(package.name.clone()),
                Some(PathBuf::from("package.json")),
            );
            diagnostics.push(ProjectPackageDiagnosticKind::CatalogTruncated, None, None);
            *catalog_incomplete = true;
            None
        }
    }
}

fn bytes_read(result: &Result<Vec<u8>, BoundedReadError>) -> usize {
    match result {
        Ok(bytes) => bytes.len(),
        Err(BoundedReadError::LimitExceeded { bytes_read })
        | Err(BoundedReadError::Failed { bytes_read }) => *bytes_read,
    }
}

fn finish_catalog(
    sources: Vec<PackageSourceDescriptor>,
    diagnostics: CatalogDiagnostics,
) -> PackageCatalog {
    PackageCatalog {
        sources,
        diagnostics: diagnostics.into_entries(),
    }
}
