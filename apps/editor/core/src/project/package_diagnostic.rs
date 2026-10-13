//! Safe, bounded diagnostic values for package catalog and source discovery.

use std::path::{Component, Path, PathBuf};

use super::packages::is_valid_technical_name;

/// A bounded, generic package discovery or content diagnostic.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum ProjectPackageDiagnosticKind {
    /// The Packages path is not a safe, project-contained directory.
    PackagesRootInvalid,
    /// The Packages directory could not be inspected.
    PackagesRootInaccessible,
    /// The direct-child limit prevented listing any embedded packages.
    CatalogOverflow,
    /// A direct package candidate was not a valid embedded package.
    CandidateInvalid,
    /// A candidate's package metadata could not be read.
    MetadataReadFailed,
    /// A candidate's package metadata was invalid.
    MetadataInvalid,
    /// A metadata byte limit stopped package inspection.
    MetadataLimit,
    /// A configured catalog limit stopped package inspection.
    CatalogTruncated,
    /// The optional project package manifest could not be used.
    ManifestInvalid,
    /// A dependency declaration did not satisfy the bounded parser contract.
    DependencyInvalid,
    /// No browsed embedded package source matched a dependency name.
    DependencyUnresolved,
    /// Some catalog diagnostics were omitted at the configured limit.
    CatalogDiagnosticsTruncated,
    /// A symbolic link was not followed.
    Symlink,
    /// A package entry escaped the package source root.
    OutsideSource,
    /// A package entry could not be inspected.
    Inaccessible,
    /// An unsupported package filesystem entry was skipped.
    UnsupportedEntry,
    /// Package traversal reached its depth limit.
    DepthLimit,
    /// Package traversal reached its filesystem inspection limit.
    InspectionLimit,
    /// The retained package entry limit was reached.
    EntryLimit,
    /// A package-relative path exceeded its individual length limit.
    PathLengthLimit,
    /// The aggregate retained package path-byte limit was reached.
    PathBytesLimit,
    /// Some package content diagnostics were omitted at the configured limit.
    ContentDiagnosticsTruncated,
    /// A discovered package could not form a consistent source index.
    SourceIndexInvalid,
}

/// A package diagnostic with only validated, bounded contextual data.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectPackageDiagnostic {
    kind: ProjectPackageDiagnosticKind,
    package_name: Option<String>,
    relative_path: Option<PathBuf>,
}

impl ProjectPackageDiagnostic {
    pub(crate) fn new(
        kind: ProjectPackageDiagnosticKind,
        package_name: Option<String>,
        relative_path: Option<PathBuf>,
    ) -> Self {
        Self {
            kind,
            package_name: package_name.filter(|name| is_valid_technical_name(name)),
            relative_path: relative_path.and_then(bounded_relative_path),
        }
    }

    /// The bounded failure classification.
    pub fn kind(&self) -> ProjectPackageDiagnosticKind {
        self.kind
    }

    /// A validated embedded package technical name, when it is safe to show.
    pub fn package_name(&self) -> Option<&str> {
        self.package_name.as_deref()
    }

    /// A bounded path relative to the diagnostic's source, when safe to show.
    pub fn relative_path(&self) -> Option<&Path> {
        self.relative_path.as_deref()
    }

    /// A fixed, generic message that does not expose filesystem or manifest data.
    pub fn message(&self) -> &'static str {
        match self.kind {
            ProjectPackageDiagnosticKind::PackagesRootInvalid => {
                "Packages path is not a safe project-contained directory."
            }
            ProjectPackageDiagnosticKind::PackagesRootInaccessible => {
                "Packages directory could not be inspected."
            }
            ProjectPackageDiagnosticKind::CatalogOverflow => {
                "Embedded package catalog exceeds the direct-child limit."
            }
            ProjectPackageDiagnosticKind::CandidateInvalid => {
                "Embedded package candidate is invalid or unavailable."
            }
            ProjectPackageDiagnosticKind::MetadataReadFailed => {
                "Embedded package metadata could not be read."
            }
            ProjectPackageDiagnosticKind::MetadataInvalid => {
                "Embedded package metadata is invalid."
            }
            ProjectPackageDiagnosticKind::MetadataLimit => {
                "Package metadata inspection reached a configured size limit."
            }
            ProjectPackageDiagnosticKind::CatalogTruncated => {
                "Embedded package catalog is incomplete due to a configured limit."
            }
            ProjectPackageDiagnosticKind::ManifestInvalid => {
                "Project package manifest is invalid or unavailable."
            }
            ProjectPackageDiagnosticKind::DependencyInvalid => {
                "A project package declaration is invalid."
            }
            ProjectPackageDiagnosticKind::DependencyUnresolved => {
                "No browsed embedded source matched; external resolution was not inspected."
            }
            ProjectPackageDiagnosticKind::CatalogDiagnosticsTruncated => {
                "Some package catalog diagnostics were omitted."
            }
            ProjectPackageDiagnosticKind::Symlink => {
                "Symbolic links are not followed during package discovery."
            }
            ProjectPackageDiagnosticKind::OutsideSource => {
                "Package entry is outside its source root."
            }
            ProjectPackageDiagnosticKind::Inaccessible => "Package entry could not be inspected.",
            ProjectPackageDiagnosticKind::UnsupportedEntry => {
                "Unsupported package filesystem entry was skipped."
            }
            ProjectPackageDiagnosticKind::DepthLimit => {
                "Package traversal reached its depth limit."
            }
            ProjectPackageDiagnosticKind::InspectionLimit => {
                "Package traversal reached its inspection limit."
            }
            ProjectPackageDiagnosticKind::EntryLimit => {
                "The retained package entry limit was reached."
            }
            ProjectPackageDiagnosticKind::PathLengthLimit => {
                "Package path exceeds the per-path limit."
            }
            ProjectPackageDiagnosticKind::PathBytesLimit => "Package path budget was exhausted.",
            ProjectPackageDiagnosticKind::ContentDiagnosticsTruncated => {
                "Some package content diagnostics were omitted."
            }
            ProjectPackageDiagnosticKind::SourceIndexInvalid => {
                "Embedded package source index could not be built."
            }
        }
    }
}

fn bounded_relative_path(path: PathBuf) -> Option<PathBuf> {
    if path.is_absolute()
        || path.components().any(|component| {
            matches!(
                component,
                Component::ParentDir | Component::RootDir | Component::Prefix(_)
            )
        })
        || path.as_os_str().as_encoded_bytes().len() > 4_096
    {
        return None;
    }

    Some(path)
}
