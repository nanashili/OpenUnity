use super::*;

impl EditorApp {
    pub(super) fn inspector_header_menu(
        &mut self,
        ui: &mut egui::Ui,
        requests: &mut Vec<InspectorHeaderAction>,
    ) {
        let menu_was_open = self
            .inspector_menu_popup
            .is_some_and(|id| egui::Popup::is_id_open(ui.ctx(), id));
        let menu = ui.menu_button("⋮", |ui| {
            if !menu_was_open {
                self.inspector_component_menu_context = self.inspector_target();
            }
            let state = self.inspector_controller.snapshot();
            let available = state.available && self.pending.is_none() && self.path_form.is_none();
            let mut checked = state.locked;
            let response = ui.add_enabled(available, egui::Checkbox::new(&mut checked, "Lock"));
            self.inspector_controls
                .push(crate::inspector_lock::control_json(
                    "inspector.menu.lock",
                    &response,
                    state.locked,
                ));
            let reason = state.reason.clone();
            if response.changed() && self.inspector_controller.snapshot().available {
                requests.push(InspectorHeaderAction::ToggleLock);
                ui.close();
                ui.ctx().request_repaint();
            }
            if let Some(reason) = reason {
                response.on_disabled_hover_text(&reason);
                ui.weak(reason);
            }
            ui.separator();
            self.inspector_expansion_menu(ui, available, requests);
        });
        self.inspector_menu_popup = Some(egui::Popup::default_response_id(&menu.response));
        menu.response.widget_info(|| {
            egui::WidgetInfo::labeled(
                egui::WidgetType::Button,
                menu.response.enabled(),
                "Inspector menu",
            )
        });
        self.inspector_controls
            .push(crate::inspector_lock::control_json(
                "inspector.menu",
                &menu.response,
                false,
            ));
    }

    fn inspector_expansion_menu(
        &mut self,
        ui: &mut egui::Ui,
        available: bool,
        requests: &mut Vec<InspectorHeaderAction>,
    ) {
        let expansion = self.component_expansion.snapshot();
        for (label, role, expanded, enabled) in [
            (
                "Expand All Components",
                "inspector.menu.expand_all",
                true,
                expansion.expand_all_enabled,
            ),
            (
                "Collapse All Components",
                "inspector.menu.collapse_all",
                false,
                expansion.collapse_all_enabled,
            ),
        ] {
            let response = ui.add_enabled(available && enabled, egui::Button::new(label));
            self.inspector_controls
                .push(crate::inspector_lock::control_json(role, &response, false));
            if response.clicked() {
                if let Some(context) = self.inspector_component_menu_context {
                    requests.push(InspectorHeaderAction::SetAllComponents { context, expanded });
                }
                ui.close();
                ui.ctx().request_repaint();
            }
            if let Some(reason) = &expansion.reason {
                response.on_disabled_hover_text(reason);
            }
        }
        if let Some(reason) = &expansion.reason {
            ui.weak(reason);
        }
    }
}
