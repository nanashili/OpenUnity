use std::path::PathBuf;

use super::super::{ProjectPackageDiagnostic, ProjectPackageDiagnosticKind};

pub(in crate::project) struct PackageCatalog {
    pub(in crate::project) sources: Vec<PackageSourceDescriptor>,
    pub(in crate::project) diagnostics: Vec<ProjectPackageDiagnostic>,
}

pub(in crate::project) struct PackageSourceDescriptor {
    pub(in crate::project) name: String,
    pub(in crate::project) display_name: Option<String>,
    pub(in crate::project) version: Option<String>,
    pub(in crate::project) hide_in_editor: bool,
    pub(in crate::project) source_root: PathBuf,
}

pub(super) struct CatalogDiagnostics {
    entries: Vec<ProjectPackageDiagnostic>,
    truncated: bool,
}

impl CatalogDiagnostics {
    pub(super) fn new() -> Self {
        Self {
            entries: Vec::new(),
            truncated: false,
        }
    }

    pub(super) fn push(
        &mut self,
        kind: ProjectPackageDiagnosticKind,
        package_name: Option<String>,
        relative_path: Option<PathBuf>,
    ) {
        const MAX_DIAGNOSTICS: usize = 1_024;
        const MAX_RETAINED_DIAGNOSTICS: usize = MAX_DIAGNOSTICS - 1;

        if self.truncated {
            return;
        }

        if self.entries.len() < MAX_RETAINED_DIAGNOSTICS {
            self.entries.push(ProjectPackageDiagnostic::new(
                kind,
                package_name,
                relative_path,
            ));
            return;
        }

        self.entries.push(ProjectPackageDiagnostic::new(
            ProjectPackageDiagnosticKind::CatalogDiagnosticsTruncated,
            None,
            None,
        ));
        self.truncated = true;
    }

    pub(super) fn into_entries(self) -> Vec<ProjectPackageDiagnostic> {
        self.entries
    }
}
