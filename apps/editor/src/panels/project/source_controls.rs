//! Source picker and hidden-package visibility control.

use super::*;

impl ProjectBrowserPanel {
    pub(super) fn show_reference_source_controls(
        &mut self,
        ui: &mut egui::Ui,
        project: &Project,
        frame: &mut ReferenceFrame,
    ) {
        self.show_source_controls(ui, project, Some(frame));
    }

    pub(super) fn show_legacy_source_controls(&mut self, ui: &mut egui::Ui, project: &Project) {
        self.show_source_controls(ui, project, None);
    }

    fn show_source_controls(
        &mut self,
        ui: &mut egui::Ui,
        project: &Project,
        frame: Option<&mut ReferenceFrame>,
    ) {
        let mut controls = Vec::new();
        ui.label("Source:");
        let source_changed = self.show_source_picker(ui, project, &mut controls);
        let hidden_source_changed = self.show_hidden_toggle(ui, project, &mut controls);

        if source_changed || hidden_source_changed {
            self.source_changed();
        }

        self.show_catalog_warning(ui, project);

        if let Some(frame) = frame {
            frame.controls.extend(controls);
        }
    }

    fn show_source_picker(
        &mut self,
        ui: &mut egui::Ui,
        project: &Project,
        controls: &mut Vec<serde_json::Value>,
    ) -> bool {
        let selected_label = source::label(project, &self.source_key);
        let mut source_changed = false;
        let response = egui::ComboBox::from_id_salt(("project.source_picker", &self.source_key))
            .selected_text(selected_label)
            .show_ui(ui, |ui| {
                let assets_selected = self.source_key == ProjectSourceKey::Assets;
                let assets = ui.selectable_label(assets_selected, "Assets");
                controls.push(appearance::control(
                    "project.source.Assets",
                    &assets,
                    assets_selected,
                ));
                if assets.clicked() && !assets_selected {
                    self.source_key = ProjectSourceKey::Assets;
                    source_changed = true;
                    ui.close();
                }

                for package in project.packages() {
                    if package.hide_in_editor() && !self.show_hidden {
                        continue;
                    }

                    let package_key = package.key();
                    let package_selected = &self.source_key == package_key;
                    let label = package
                        .display_name()
                        .filter(|display_name| !display_name.is_empty())
                        .unwrap_or_else(|| package.name());
                    let row = ui.selectable_label(package_selected, label);
                    controls.push(appearance::control(
                        source::source_role(package_key),
                        &row,
                        package_selected,
                    ));
                    let row = row.on_hover_text(package_tooltip(package));

                    if row.clicked() && !package_selected {
                        self.source_key = package_key.clone();
                        source_changed = true;
                        ui.close();
                    }
                }
            });
        controls.push(appearance::control(
            source::source_role(&self.source_key),
            &response.response,
            false,
        ));

        source_changed
    }

    fn show_hidden_toggle(
        &mut self,
        ui: &mut egui::Ui,
        project: &Project,
        controls: &mut Vec<serde_json::Value>,
    ) -> bool {
        let hidden_count = project
            .packages()
            .iter()
            .filter(|package| package.hide_in_editor())
            .count();
        let label = format!("Show Hidden ({hidden_count})");
        let response = ui
            .push_id(("project.show_hidden", &self.source_key), |ui| {
                ui.checkbox(&mut self.show_hidden, label)
            })
            .inner;
        let response = response.on_hover_text(format!(
            "Show embedded packages marked hideInEditor=true. {hidden_count} package(s) are hidden by default."
        ));
        controls.push(appearance::control(
            "project.show_hidden",
            &response,
            self.show_hidden,
        ));

        let source_changed = !self.show_hidden
            && matches!(
                &self.source_key,
                ProjectSourceKey::EmbeddedPackage(name)
                    if project.packages().iter().any(|package| {
                        package.name() == name && package.hide_in_editor()
                    })
            );
        if source_changed {
            self.source_key = ProjectSourceKey::Assets;
        }

        source_changed
    }

    fn show_catalog_warning(&self, ui: &mut egui::Ui, project: &Project) {
        let count = project.package_diagnostics().len();
        if count == 0 {
            return;
        }

        let message = format!(
            "Package manifest or embedded-source discovery reported {count} diagnostic{}. External package registry resolution is not inspected.",
            if count == 1 { "" } else { "s" }
        );
        ui.colored_label(egui::Color32::YELLOW, "Package source diagnostics")
            .on_hover_text(message);
    }
}

fn package_tooltip(package: &openunity_editor_core::project::ProjectPackage) -> String {
    let mut details = vec![package.name().to_owned()];

    if let Some(version) = package.version() {
        details.push(format!("Version: {version}"));
    }
    if package.hide_in_editor() {
        details.push("Hidden by default (hideInEditor=true)".to_owned());
    }

    details.join("\n")
}
