//! Legacy project metadata, ranked path rows and selected-entry details.

use super::*;

impl ProjectBrowserPanel {
    /// Shows source metadata, searchable entries, and read-only selection details.
    pub fn show(&mut self, ui: &mut egui::Ui, project: Option<&Project>) {
        if appearance::is_reference(ui) {
            self.show_reference(ui, project);
            return;
        }
        self.show_legacy(ui, project);
    }

    fn show_legacy(&mut self, ui: &mut egui::Ui, project: Option<&Project>) {
        ui.heading("Project");
        let Some(project) = project else {
            self.project_root = None;
            self.selected_path = None;
            ui.label("No project is open.");
            ui.label("Open a project directory containing Assets and ProjectSettings/ProjectVersion.txt.");
            return;
        };

        self.reconcile_legacy_project(project);
        self.show_legacy_controls(ui, project);
        let labels: Vec<_> = project
            .entries()
            .iter()
            .map(|entry| entry.relative_path.to_string_lossy())
            .collect();
        let matching = ranked_indices(&self.filter, labels.iter().map(|label| label.as_ref()));
        self.show_legacy_matches(ui, project, &labels, &matching);
        self.show_legacy_selection(ui, project);
    }

    fn show_legacy_controls(&mut self, ui: &mut egui::Ui, project: &Project) {
        ui.label(format!("Root: {}", project.root().display()));
        ui.label(format!(
            "Declared editor version: {}",
            project.editor_version()
        ));
        ui.label(format!(
            "Discovery diagnostics: {}",
            project.diagnostics().len()
        ));
        ui.label("Read-only source snapshot. Files have not been imported.");
        ui.horizontal(|ui| {
            let label = ui.label("Filter Assets paths:");
            ui.add(egui::TextEdit::singleline(&mut self.filter).hint_text("Search Assets paths"))
                .labelled_by(label.id);
            if ui.button("Clear filter").clicked() {
                self.filter.clear();
            }
        });
    }

    fn show_legacy_matches(
        &mut self,
        ui: &mut egui::Ui,
        project: &Project,
        labels: &[std::borrow::Cow<'_, str>],
        matching: &[usize],
    ) {
        ui.label(format!(
            "{} of {} entries",
            matching.len(),
            project.entries().len()
        ));
        if project.entries().is_empty() {
            ui.label("Assets contains no discovered files or directories.");
        } else if matching.is_empty() {
            ui.label("No entries match this filter.");
        } else {
            self.show_legacy_rows(ui, project, labels, matching);
        }
    }

    fn show_legacy_rows(
        &mut self,
        ui: &mut egui::Ui,
        project: &Project,
        labels: &[std::borrow::Cow<'_, str>],
        matching: &[usize],
    ) {
        // The reader bounds the snapshot; virtual rows keep rendering bounded
        // to the visible portion even when discovery reaches its entry limit.
        let row_height = ui
            .text_style_height(&egui::TextStyle::Body)
            .max(ui.spacing().interact_size.y);
        egui::ScrollArea::vertical()
            .id_salt("project_entries")
            .max_height((ui.available_height() - 110.0).max(row_height * 2.0))
            .show_rows(ui, row_height, matching.len(), |ui, rows| {
                for row in rows {
                    let index = matching[row];
                    let entry = &project.entries()[index];
                    let kind = entry_kind(entry.kind);
                    let selected =
                        self.selected_path.as_deref() == Some(entry.relative_path.as_path());
                    if ui
                        .selectable_label(selected, format!("{} ({kind})", labels[index]))
                        .clicked()
                    {
                        self.selected_path = Some(entry.relative_path.clone());
                    }
                }
            });
    }

    fn show_legacy_selection(&self, ui: &mut egui::Ui, project: &Project) {
        ui.separator();
        if let Some(entry) = project
            .entries()
            .iter()
            .find(|entry| self.selected_path.as_deref() == Some(entry.relative_path.as_path()))
        {
            ui.label(format!("Relative path: {}", entry.relative_path.display()));
            ui.label(format!("Type: {}", entry_kind(entry.kind)));
            ui.label("Status: discovered on disk; read-only; not imported.");
        } else {
            ui.label("Select a file or directory to view its path and discovery status.");
        }
    }
}
