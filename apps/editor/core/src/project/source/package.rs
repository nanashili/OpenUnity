//! Embedded package metadata and its bounded source-index content.

use std::path::PathBuf;

use super::super::ProjectIndex;
use super::super::packages::PackageSourceDescriptor;
use super::super::{ProjectEntry, ProjectPackageDiagnostic};
use super::ProjectSourceKey;

/// Entries and content diagnostics produced by one bounded package walk.
#[derive(Debug)]
pub(crate) struct PackageContent {
    entries: Vec<ProjectEntry>,
    diagnostics: Vec<ProjectPackageDiagnostic>,
}

impl PackageContent {
    pub(crate) fn new(
        entries: Vec<ProjectEntry>,
        diagnostics: Vec<ProjectPackageDiagnostic>,
    ) -> Self {
        Self {
            entries,
            diagnostics,
        }
    }
}

/// Metadata and source index for one valid embedded package.
#[derive(Debug)]
pub struct ProjectPackage {
    pub(super) key: ProjectSourceKey,
    display_name: Option<String>,
    version: Option<String>,
    hide_in_editor: bool,
    pub(super) root: PathBuf,
    pub(super) entries: Vec<ProjectEntry>,
    pub(super) index: ProjectIndex,
    pub(super) diagnostics: Vec<ProjectPackageDiagnostic>,
}

impl ProjectPackage {
    pub(in crate::project) fn from_discovery(
        descriptor: PackageSourceDescriptor,
        content: PackageContent,
    ) -> Result<Self, usize> {
        let index = ProjectIndex::build(&content.entries)?;

        Ok(Self {
            key: ProjectSourceKey::EmbeddedPackage(descriptor.name),
            display_name: descriptor.display_name,
            version: descriptor.version,
            hide_in_editor: descriptor.hide_in_editor,
            root: descriptor.source_root,
            entries: content.entries,
            index,
            diagnostics: content.diagnostics,
        })
    }

    /// The stable technical package identity.
    pub fn key(&self) -> &ProjectSourceKey {
        &self.key
    }

    /// The technical package name used in logical paths.
    pub fn name(&self) -> &str {
        match &self.key {
            ProjectSourceKey::EmbeddedPackage(name) => name,
            ProjectSourceKey::Assets => unreachable!("package records always use package keys"),
        }
    }

    /// The optional display label from package metadata.
    pub fn display_name(&self) -> Option<&str> {
        self.display_name.as_deref()
    }

    /// The optional version token from package metadata.
    pub fn version(&self) -> Option<&str> {
        self.version.as_deref()
    }

    /// Whether the package is hidden by the editor's default package filter.
    pub fn hide_in_editor(&self) -> bool {
        self.hide_in_editor
    }
}
