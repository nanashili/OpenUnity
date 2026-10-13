//! Project identity, selection and current-directory reconciliation.

use super::*;

impl ProjectBrowserPanel {
    /// The selected Assets-relative path, scoped to the most recently shown project.
    pub fn selected_path(&self) -> Option<&Path> {
        self.selected_path.as_deref()
    }

    pub(super) fn reconcile_legacy_project(&mut self, project: &Project) {
        if self.project_root.as_deref() != Some(project.root()) {
            self.project_root = Some(project.root().to_path_buf());
            self.selected_path = None;
        }
        if self
            .selected_path
            .as_ref()
            .is_some_and(|selected| project.entry(selected).is_none())
        {
            self.selected_path = None;
        }
    }

    pub(super) fn reconcile_reference_project(&mut self, project: Option<&Project>) {
        if self.project_root.as_deref() != project.map(Project::root) {
            self.project_root = project.map(|p| p.root().to_path_buf());
            self.selected_path = None;
            self.directory.clear();
        }
        if let Some(project) = project
            && self
                .selected_path
                .as_ref()
                .is_some_and(|selected| project.entry(selected).is_none())
        {
            self.selected_path = None;
        }
    }

    pub(super) fn reconcile_directory(&mut self, project: &Project) {
        if !self.directory.as_os_str().is_empty()
            && project
                .entry(&self.directory)
                .is_none_or(|entry| entry.kind != ProjectEntryKind::Directory)
        {
            self.directory.clear();
        }
    }
}
