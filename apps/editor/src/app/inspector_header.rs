use super::*;

impl EditorApp {
    /// Render the production Reference Inspector action area inside an allocated header.
    /// The workspace reserves this area outside the tab-title hit rectangle.
    /// Returns activation requests for ordered dispatch against committed state.
    pub fn show_inspector_header(&mut self, ui: &mut egui::Ui) -> Vec<InspectorHeaderAction> {
        if self.workspace.profile() != AppearanceProfile::Reference {
            return Vec::new();
        }
        let mut requests = Vec::new();
        ui.spacing_mut().item_spacing.x = 0.0;
        ui.spacing_mut().button_padding = egui::Vec2::ZERO;
        let available = self.inspector_controller.snapshot().available
            && self.pending.is_none()
            && self.path_form.is_none();
        self.inspector_lock_control(ui, available, &mut requests);
        self.inspector_header_menu(ui, &mut requests);
        requests
    }

    fn inspector_lock_control(
        &mut self,
        ui: &mut egui::Ui,
        available: bool,
        requests: &mut Vec<InspectorHeaderAction>,
    ) {
        let response = ui
            .add_enabled_ui(available, |ui| {
                let response = ui.add_sized(
                    [24.0, 24.0],
                    egui::Button::new("").selected(self.inspector_controller.snapshot().locked),
                );
                if response.clicked() && self.inspector_controller.snapshot().available {
                    requests.push(InspectorHeaderAction::ToggleLock);
                    ui.ctx().request_repaint();
                }
                let state = self.inspector_controller.snapshot();
                response.widget_info(|| {
                    egui::WidgetInfo::selected(
                        egui::WidgetType::Checkbox,
                        response.enabled(),
                        state.locked,
                        "Lock Inspector",
                    )
                });
                let icon_rect =
                    egui::Rect::from_center_size(response.rect.center(), egui::vec2(16.0, 16.0));
                crate::resources::paint(
                    ui,
                    if state.locked {
                        "lock_locked"
                    } else {
                        "lock_unlocked"
                    },
                    "inspector.lock",
                    response.id,
                    icon_rect,
                );
                response
            })
            .inner;
        self.record_inspector_lock(response);
    }

    fn record_inspector_lock(&mut self, response: egui::Response) {
        let state = self.inspector_controller.snapshot();
        let icon_rect =
            egui::Rect::from_center_size(response.rect.center(), egui::vec2(16.0, 16.0));
        let mut control =
            crate::inspector_lock::control_json("inspector.lock", &response, state.locked);
        control["icon_rect"] = appearance::rect_json(icon_rect);
        control["accessible_name"] = "Lock Inspector".into();
        control["accessible_pressed"] = state.locked.into();
        control["reason"] = state.reason.clone().into();
        self.inspector_controls.push(control);
        response
            .on_hover_text(if state.locked {
                "Unlock Inspector"
            } else {
                "Lock Inspector"
            })
            .on_disabled_hover_text(
                state
                    .reason
                    .as_deref()
                    .unwrap_or("Inspector actions unavailable while a document dialog is open."),
            );
    }
}
