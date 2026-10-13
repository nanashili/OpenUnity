//! Shared resource budgets for all embedded package content indexes.

use std::path::Path;

use super::super::super::{
    MAX_PROJECT_ENTRIES, ProjectPackageDiagnostic, ProjectPackageDiagnosticKind,
};

const MAX_PACKAGE_INSPECTED_ENTRIES: usize = 100_000;
const MAX_PACKAGE_PATH_BYTES: usize = 16 * 1024 * 1024;
const MAX_PACKAGE_CONTENT_DIAGNOSTICS: usize = 1_024;
const MAX_SOURCE_CONTENT_DIAGNOSTICS: usize = 256;

pub(in crate::project) struct PackageContentBudget {
    inspected_entries: usize,
    retained_entries: usize,
    retained_path_bytes: usize,
    diagnostics: usize,
    remaining_sources: usize,
    stopped_by: Option<ProjectPackageDiagnosticKind>,
}

impl PackageContentBudget {
    pub(in crate::project) fn new(assets_entry_count: usize, package_count: usize) -> Self {
        Self {
            inspected_entries: 0,
            retained_entries: assets_entry_count,
            retained_path_bytes: 0,
            diagnostics: 0,
            remaining_sources: package_count,
            stopped_by: (assets_entry_count >= MAX_PROJECT_ENTRIES)
                .then_some(ProjectPackageDiagnosticKind::EntryLimit),
        }
    }

    pub(super) fn begin_source(&mut self, package_name: &str) -> PackageContentState<'_> {
        self.remaining_sources = self.remaining_sources.saturating_sub(1);
        PackageContentState {
            budget: self,
            package_name: package_name.to_owned(),
            diagnostics: Vec::new(),
            diagnostics_truncated: false,
        }
    }
}

pub(super) struct PackageContentState<'a> {
    budget: &'a mut PackageContentBudget,
    package_name: String,
    diagnostics: Vec<ProjectPackageDiagnostic>,
    diagnostics_truncated: bool,
}

impl PackageContentState<'_> {
    pub(super) fn can_inspect_next(&mut self, relative_directory: &Path) -> bool {
        if self.budget.stopped_by.is_some() {
            return false;
        }

        if self.budget.inspected_entries >= MAX_PACKAGE_INSPECTED_ENTRIES {
            self.stop(
                ProjectPackageDiagnosticKind::InspectionLimit,
                Some(relative_directory),
            );
            return false;
        }

        true
    }

    pub(super) fn record_inspected_entry(&mut self) {
        self.budget.inspected_entries += 1;
    }

    pub(super) fn retain_entry(&mut self, relative_path: &Path) -> bool {
        if self.budget.stopped_by.is_some() {
            return false;
        }

        if self.budget.retained_entries >= MAX_PROJECT_ENTRIES {
            self.stop(
                ProjectPackageDiagnosticKind::EntryLimit,
                Some(relative_path),
            );
            return false;
        }

        let path_bytes = relative_path.as_os_str().as_encoded_bytes().len();
        if self.budget.retained_path_bytes.saturating_add(path_bytes) > MAX_PACKAGE_PATH_BYTES {
            self.stop(
                ProjectPackageDiagnosticKind::PathBytesLimit,
                Some(relative_path),
            );
            return false;
        }

        self.budget.retained_entries += 1;
        self.budget.retained_path_bytes += path_bytes;
        true
    }

    pub(super) fn stop(
        &mut self,
        kind: ProjectPackageDiagnosticKind,
        relative_path: Option<&Path>,
    ) {
        if self.budget.stopped_by.is_none() {
            self.budget.stopped_by = Some(kind);
            self.push_diagnostic(
                kind,
                Some(self.package_name.clone()),
                relative_path.map(Path::to_path_buf),
            );
        }
    }

    pub(super) fn stop_reason(&self) -> Option<ProjectPackageDiagnosticKind> {
        self.budget.stopped_by
    }

    pub(super) fn diagnose(
        &mut self,
        kind: ProjectPackageDiagnosticKind,
        relative_path: Option<&Path>,
    ) {
        self.push_diagnostic(
            kind,
            Some(self.package_name.clone()),
            relative_path.map(Path::to_path_buf),
        );
    }

    pub(super) fn into_diagnostics(self) -> Vec<ProjectPackageDiagnostic> {
        self.diagnostics
    }

    fn push_diagnostic(
        &mut self,
        kind: ProjectPackageDiagnosticKind,
        package_name: Option<String>,
        relative_path: Option<std::path::PathBuf>,
    ) {
        if self.diagnostics_truncated {
            return;
        }

        let source_limit = MAX_SOURCE_CONTENT_DIAGNOSTICS;
        let source_retained_limit = source_limit - 1;
        let total_limit = MAX_PACKAGE_CONTENT_DIAGNOSTICS - self.budget.remaining_sources;
        let total_retained_limit = total_limit.saturating_sub(1);

        if self.diagnostics.len() >= source_retained_limit
            || self.budget.diagnostics >= total_retained_limit
        {
            if self.budget.diagnostics < total_limit {
                self.diagnostics.push(ProjectPackageDiagnostic::new(
                    ProjectPackageDiagnosticKind::ContentDiagnosticsTruncated,
                    None,
                    None,
                ));
                self.budget.diagnostics += 1;
            }
            self.diagnostics_truncated = true;
            return;
        }

        self.diagnostics.push(ProjectPackageDiagnostic::new(
            kind,
            package_name,
            relative_path,
        ));
        self.budget.diagnostics += 1;
    }
}
