use super::*;

impl EditorApp {
    pub(super) fn hierarchy_pane(
        &mut self,
        ui: &mut egui::Ui,
        reference: bool,
        pending: &mut PanelActions,
    ) {
        if reference {
            if let Some(document) = &self.imported {
                pending
                    .imported_actions
                    .extend(imported::show_hierarchy_reference(
                        ui,
                        document,
                        &mut self.hierarchy_browser,
                    ));
            } else {
                pending.actions.extend(hierarchy::show_reference(
                    ui,
                    self.document.scene(),
                    self.document.selection(),
                    &mut self.hierarchy_browser,
                ));
            }
        } else {
            egui::ScrollArea::vertical()
                .id_salt("workspace.hierarchy.scroll")
                .show(ui, |ui| {
                    if let Some(document) = &self.imported {
                        pending
                            .imported_actions
                            .extend(imported::show_hierarchy(ui, document));
                    } else {
                        pending.actions.extend(hierarchy::show(
                            ui,
                            self.document.scene(),
                            self.document.selection(),
                        ));
                    }
                });
        }
    }

    pub(super) fn scene_pane(&mut self, ui: &mut egui::Ui, pending: &mut PanelActions) {
        if let Some(document) = &self.imported {
            pending
                .imported_actions
                .extend(self.imported_viewport.show(ui, document));
        } else {
            pending.actions.extend(self.viewport_panel.show(
                ui,
                self.document.scene(),
                self.document.selection(),
            ));
        }
    }

    pub(super) fn game_pane(&mut self, ui: &mut egui::Ui, reference: bool) {
        if reference {
            let rect = ui.available_rect_before_wrap();
            ui.painter().rect_filled(
                rect.expand2(egui::vec2(4.0, 0.0)),
                0.0,
                egui::Color32::from_rgb(55, 76, 118),
            );
            egui::Frame::NONE
                .fill(appearance::BAR)
                .inner_margin(4.0)
                .show(ui, |ui| {
                    ui.set_width((rect.width() - 8.0).max(1.0));
                    ui.horizontal(|ui| {
                        ui.label("Game rendering unavailable").on_hover_text(
                            "Empty design canvas only. No game renderer, camera output or Play execution is available. Switch to Scene for the actual Transform-only preview.",
                        );
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.add_enabled(false, egui::Button::new("Display"))
                                .on_disabled_hover_text("Game displays are unsupported.");
                        });
                    });
                });
        } else {
            ui.heading("Game rendering is unsupported");
            ui.label("This editor provides independent authoring previews. No game renderer or Play execution is available.");
        }
    }
}
