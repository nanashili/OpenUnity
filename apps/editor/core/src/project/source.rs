//! Borrowed source views over the entries retained in one Project snapshot.

mod content;
mod package;
mod view;

use super::{Project, ProjectPackageDiagnostic};

static ASSETS_SOURCE_KEY: ProjectSourceKey = ProjectSourceKey::Assets;

/// Stable identity for one logical source in a project snapshot.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum ProjectSourceKey {
    /// The project's existing Assets directory.
    Assets,
    /// An embedded package identified by its technical package name.
    EmbeddedPackage(String),
}

pub(super) use content::{PackageContentBudget, discover_package_content};
pub(super) use package::PackageContent;
pub use package::ProjectPackage;
pub use view::{ProjectSource, ProjectSourceDiagnostics};

impl Project {
    /// Valid embedded package records, ordered by technical name.
    pub fn packages(&self) -> &[ProjectPackage] {
        &self.packages
    }

    /// Catalog-level diagnostics for bounded embedded-package metadata discovery.
    pub fn package_diagnostics(&self) -> &[ProjectPackageDiagnostic] {
        &self.package_diagnostics
    }

    /// Selects Assets with `None`, or an embedded package by exact technical name.
    pub fn source(&self, package_name: Option<&str>) -> Option<ProjectSource<'_>> {
        let Some(package_name) = package_name else {
            return Some(ProjectSource::new(
                &ASSETS_SOURCE_KEY,
                &self.assets_root,
                &self.entries,
                &self.index,
                ProjectSourceDiagnostics::Assets(&self.diagnostics),
            ));
        };

        let package_index = self
            .packages
            .binary_search_by(|package| package.name().cmp(package_name))
            .ok()?;
        let package = &self.packages[package_index];

        Some(ProjectSource::new(
            &package.key,
            &package.root,
            &package.entries,
            &package.index,
            ProjectSourceDiagnostics::EmbeddedPackage(&package.diagnostics),
        ))
    }
}
