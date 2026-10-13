//! Assemble accepted package metadata with its bounded content index.

use std::path::Path;

use super::super::packages::{PackageCatalog, PackageSourceDescriptor};
use super::super::source::{PackageContentBudget, ProjectPackage, discover_package_content};
use super::super::{ProjectPackageDiagnostic, ProjectPackageDiagnosticKind};

pub(super) fn discover_package_sources(
    project_root: &Path,
    assets_entry_count: usize,
    catalog: PackageCatalog,
) -> (Vec<ProjectPackage>, Vec<ProjectPackageDiagnostic>) {
    let PackageCatalog {
        sources,
        mut diagnostics,
    } = catalog;
    let mut budget = PackageContentBudget::new(assets_entry_count, sources.len());
    let mut packages = Vec::with_capacity(sources.len());

    for descriptor in sources {
        append_package_source(
            descriptor,
            project_root,
            &mut budget,
            &mut packages,
            &mut diagnostics,
        );
    }

    (packages, diagnostics)
}

fn append_package_source(
    descriptor: PackageSourceDescriptor,
    project_root: &Path,
    budget: &mut PackageContentBudget,
    packages: &mut Vec<ProjectPackage>,
    diagnostics: &mut Vec<ProjectPackageDiagnostic>,
) {
    let package_name = descriptor.name.clone();
    let content = discover_package_content(&descriptor, project_root, budget);

    match ProjectPackage::from_discovery(descriptor, content) {
        Ok(package) => packages.push(package),
        Err(_) => record_index_failure(diagnostics, package_name),
    }
}

fn record_index_failure(diagnostics: &mut Vec<ProjectPackageDiagnostic>, package_name: String) {
    const MAX_CATALOG_DIAGNOSTICS: usize = 1_024;
    const MAX_RETAINED_FAILURES: usize = MAX_CATALOG_DIAGNOSTICS - 1;

    if diagnostics.len() >= MAX_CATALOG_DIAGNOSTICS {
        return;
    }

    if diagnostics.len() >= MAX_RETAINED_FAILURES {
        diagnostics.push(ProjectPackageDiagnostic::new(
            ProjectPackageDiagnosticKind::CatalogDiagnosticsTruncated,
            None,
            None,
        ));
        return;
    }

    diagnostics.push(ProjectPackageDiagnostic::new(
        ProjectPackageDiagnosticKind::SourceIndexInvalid,
        Some(package_name),
        None,
    ));
}
