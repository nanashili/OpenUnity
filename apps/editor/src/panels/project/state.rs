//! Project identity, selection and current-directory reconciliation.

use super::*;

impl ProjectBrowserPanel {
    /// The selected path relative to the active project source.
    pub fn selected_path(&self) -> Option<&Path> {
        self.selected_path.as_deref()
    }

    pub(super) fn reconcile_legacy_project(&mut self, project: &Project) {
        if self.project_root.as_deref() != Some(project.root()) {
            self.project_root = Some(project.root().to_path_buf());
            self.source_key = ProjectSourceKey::Assets;
            self.source_changed();
        }

        self.reconcile_source(project);

        if let Some(source) = source::resolve(project, &self.source_key) {
            self.reconcile_selected_entry(source);
        }
    }

    pub(super) fn reconcile_reference_project(&mut self, project: Option<&Project>) {
        if self.project_root.as_deref() != project.map(Project::root) {
            self.project_root = project.map(|p| p.root().to_path_buf());
            self.source_key = ProjectSourceKey::Assets;
            self.source_changed();
        }

        let Some(project) = project else {
            self.source_key = ProjectSourceKey::Assets;
            self.source_changed();
            return;
        };

        self.reconcile_source(project);

        if let Some(source) = source::resolve(project, &self.source_key) {
            self.reconcile_selected_entry(source);
            self.reconcile_directory(source);
        }
    }

    pub(super) fn source_changed(&mut self) {
        self.selected_path = None;
        self.selected_path_kind = None;
        self.directory.clear();
        self.query_cache.clear();
        self.last_frame = None;
        self.frozen_snapshot = None;
    }

    fn reconcile_source(&mut self, project: &Project) {
        let source_exists = source::resolve(project, &self.source_key).is_some();
        let source_is_visible = match &self.source_key {
            ProjectSourceKey::Assets => true,
            ProjectSourceKey::EmbeddedPackage(name) => project
                .packages()
                .iter()
                .find(|package| package.name() == name)
                .is_some_and(|package| self.show_hidden || !package.hide_in_editor()),
        };

        if !source_exists || !source_is_visible {
            self.source_key = ProjectSourceKey::Assets;
            self.source_changed();
        }
    }

    fn reconcile_selected_entry(&mut self, source: ProjectSource<'_>) {
        let Some(selected) = self.selected_path.as_deref() else {
            self.selected_path_kind = None;
            return;
        };

        let still_same_entry = source
            .entry(selected)
            .is_some_and(|entry| Some(entry.kind) == self.selected_path_kind);
        if !still_same_entry {
            self.selected_path = None;
            self.selected_path_kind = None;
        }
    }

    pub(super) fn reconcile_directory(&mut self, source: ProjectSource<'_>) {
        if !self.directory.as_os_str().is_empty()
            && source
                .entry(&self.directory)
                .is_none_or(|entry| entry.kind != ProjectEntryKind::Directory)
        {
            self.directory.clear();
        }
    }
}
