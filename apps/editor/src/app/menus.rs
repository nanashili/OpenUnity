use super::*;

impl EditorApp {
    pub(super) fn menu_item(
        &mut self,
        ui: &mut egui::Ui,
        label: &str,
        role: &str,
        enabled: bool,
    ) -> bool {
        let response = ui.add_enabled(enabled, egui::Button::new(label));
        self.workspace.record(role, &response, false);
        if response.clicked() {
            ui.close();
            true
        } else {
            false
        }
    }

    pub(super) fn file_menu(&mut self, ui: &mut egui::Ui) {
        let file = ui.menu_button("File", |ui| {
            if self.menu_item(ui, "New", "command.new", true) {
                self.command(Command::NewScene);
            }
            if self.menu_item(ui, "Open Project…", "command.open_project", true) {
                self.open_form(PathFormKind::OpenProject);
            }
            if self.menu_item(ui, "Open Scene/Archive…", "command.open_scene", true) {
                self.open_form(PathFormKind::OpenScene);
            }
            if self.menu_item(
                ui,
                "Import Saved Scene…",
                "command.import",
                self.project.is_some(),
            ) {
                self.open_form(PathFormKind::Import);
            }
            ui.separator();
            if self.menu_item(ui, "Save", "command.save", true) {
                if self.save_path.is_some() {
                    let result = self.save_current();
                    self.report(result, "Scene saved.");
                } else {
                    self.open_form(PathFormKind::SaveAs);
                }
            }
            if self.menu_item(ui, "Save As…", "command.save_as", true) {
                self.open_form(PathFormKind::SaveAs);
            }
            ui.separator();
            if self.menu_item(ui, "Close", "command.close", true) {
                let result = self.request_close();
                self.report(result, "Close requested.");
            }
        });
        self.workspace.record("menu.file", &file.response, false);
    }

    pub(super) fn edit_menu(&mut self, ui: &mut egui::Ui) {
        let edit = ui.menu_button("Edit", |ui| {
            if self.menu_item(ui, "Undo", "command.undo", self.can_undo()) {
                self.command(Command::Undo);
            }
            if self.menu_item(ui, "Redo", "command.redo", self.can_redo()) {
                self.command(Command::Redo);
            }
        });
        self.workspace.record("menu.edit", &edit.response, false);
    }

    pub(super) fn view_menu(&mut self, ui: &mut egui::Ui, reference: bool) {
        let view = ui.menu_button("View", |ui| {
            if self.menu_item(ui, "Reset Workspace", "command.reset", true) {
                self.workspace.reset();
            }
            let grid = ui.checkbox(&mut self.preferences.grid_visible, "World grid");
            self.workspace
                .record("command.grid", &grid, self.preferences.grid_visible);
            if reference && self.menu_item(ui, "Build…", "command.build", true) {
                self.report(
                    capabilities::attempt(Capability::Build).map_err(|e| e.to_string()),
                    "",
                );
            }
        });
        self.workspace.record("menu.view", &view.response, false);
    }
}
