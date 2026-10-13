//! Legacy source metadata, ranked path rows and selected-entry details.

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
            self.reset_for_missing_project();
            ui.label("No project is open.");
            ui.label(
                "Open a project directory containing Assets and ProjectSettings/ProjectVersion.txt.",
            );
            return;
        };

        self.reconcile_legacy_project(project);
        self.show_legacy_controls(ui, project);

        let Some(source) = source::resolve(project, &self.source_key) else {
            return;
        };
        self.show_incomplete_source_warning(ui, source);

        let (matching, total_visible_entries) =
            self.query_cache
                .legacy_indices(project, source, &self.filter);
        self.show_legacy_matches(ui, source, &matching, total_visible_entries);
        self.show_legacy_selection(ui, source);
    }

    fn reset_for_missing_project(&mut self) {
        self.project_root = None;
        self.source_key = ProjectSourceKey::Assets;
        self.source_changed();
    }

    fn show_legacy_controls(&mut self, ui: &mut egui::Ui, project: &Project) {
        self.show_legacy_source_controls(ui, project);
        let active_source_label = source::label(project, &self.source_key);
        ui.label(format!("Root: {}", project.root().display()));
        ui.label(format!(
            "Active source: {}",
            source::logical_root(&self.source_key)
        ));
        ui.label(format!(
            "Declared editor version: {}",
            project.editor_version()
        ));
        ui.label(format!(
            "Assets discovery diagnostics: {}",
            project.diagnostics().len()
        ));
        ui.label("Read-only source snapshot. Files have not been imported.")
            .on_hover_text(format!("Selected source label: {active_source_label}"));
        let filter_label = source::legacy_filter_label(&self.source_key);
        let search_hint = source::legacy_search_hint(&self.source_key);
        let source_scope = source::source_role(&self.source_key);
        ui.push_id(source_scope, |ui| {
            ui.horizontal(|ui| {
                let label = ui.label(&filter_label);
                ui.add(egui::TextEdit::singleline(&mut self.filter).hint_text(search_hint))
                    .labelled_by(label.id);
                if ui.button("Clear filter").clicked() {
                    self.filter.clear();
                }
            });
        });
    }

    fn show_incomplete_source_warning(&self, ui: &mut egui::Ui, source: ProjectSource<'_>) {
        if let Some(warning) = source::incomplete_warning(source) {
            ui.colored_label(egui::Color32::YELLOW, "Active source may be incomplete")
                .on_hover_text(warning);
        }
    }

    fn show_legacy_matches(
        &mut self,
        ui: &mut egui::Ui,
        source: ProjectSource<'_>,
        matching: &super::cache::CachedEntryIndices,
        total_visible_entries: usize,
    ) {
        let matching_count = matching.legacy_len(source);
        ui.label(format!(
            "{} of {} entries",
            matching_count, total_visible_entries
        ))
        .on_hover_text("Counts omit hidden .meta sidecar files.");
        if total_visible_entries == 0 {
            if source.entries().is_empty() {
                ui.label(format!(
                    "{} contains no discovered files or directories.",
                    source::logical_root(source.key())
                ));
            } else {
                ui.label("This source contains no visible files or directories.");
            }
        } else if matching_count == 0 {
            ui.label("No entries match this filter.");
        } else {
            self.show_legacy_rows(ui, source, matching);
        }
    }

    fn show_legacy_rows(
        &mut self,
        ui: &mut egui::Ui,
        source: ProjectSource<'_>,
        matching: &super::cache::CachedEntryIndices,
    ) {
        let row_height = ui
            .text_style_height(&egui::TextStyle::Body)
            .max(ui.spacing().interact_size.y);
        egui::ScrollArea::vertical()
            .id_salt(("project_entries", source.key()))
            .max_height((ui.available_height() - 110.0).max(row_height * 2.0))
            .show_rows(ui, row_height, matching.legacy_len(source), |ui, rows| {
                for row in rows {
                    let Some(index) = matching.legacy_index(source, row) else {
                        continue;
                    };
                    let Some(entry) = source.entries().get(index) else {
                        continue;
                    };

                    self.show_legacy_row(ui, source, entry);
                }
            });
    }

    fn show_legacy_row(
        &mut self,
        ui: &mut egui::Ui,
        source: ProjectSource<'_>,
        entry: &ProjectEntry,
    ) {
        let kind = entry_kind(entry.kind);
        let selected = self.selected_path.as_deref() == Some(entry.relative_path.as_path());
        let label = source::legacy_entry_label(source.key(), &entry.relative_path);
        let response = ui
            .push_id(source.key(), |ui| {
                ui.selectable_label(selected, format!("{label} ({kind})"))
            })
            .inner;

        if response.clicked() {
            self.selected_path = Some(entry.relative_path.clone());
            self.selected_path_kind = Some(entry.kind);
        }
        response.on_hover_text(source::logical_path(source.key(), &entry.relative_path));
    }

    fn show_legacy_selection(&self, ui: &mut egui::Ui, source: ProjectSource<'_>) {
        ui.separator();
        if let Some(entry) = self
            .selected_path
            .as_deref()
            .and_then(|path| source.entry(path))
            .filter(|entry| Some(entry.kind) == self.selected_path_kind)
        {
            ui.label(format!("Relative path: {}", entry.relative_path.display()));
            ui.label(format!(
                "Source path: {}",
                source::logical_path(source.key(), &entry.relative_path)
            ));
            ui.label(format!("Type: {}", entry_kind(entry.kind)));
            ui.label("Status: discovered on disk; read-only; not imported.");
        } else {
            ui.label("Select a file or directory to view its path and discovery status.");
        }
    }
}
