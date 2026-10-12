use super::*;

impl EditorApp {
    pub(super) fn transport(&mut self, ui: &mut egui::Ui, enabled: bool) {
        let rect = ui.max_rect();
        let mut transport = ui.new_child(
            egui::UiBuilder::new()
                .id_salt("workspace.transport")
                .max_rect(egui::Rect::from_center_size(
                    rect.center(),
                    egui::vec2(94.0, 22.0),
                ))
                .layout(egui::Layout::left_to_right(egui::Align::Center)),
        );
        transport.spacing_mut().item_spacing.x = 1.0;
        for (role, label, reason) in [
            ("command.play", "Play", "Play execution is unsupported."),
            (
                "command.pause",
                "Pause",
                "Pause requires unsupported Play execution.",
            ),
            (
                "command.step",
                "Step",
                "Step requires unsupported Play execution.",
            ),
        ] {
            let response = transport
                .add_enabled(
                    false,
                    egui::Button::new("").min_size(egui::vec2(29.0, 20.0)),
                )
                .on_disabled_hover_text(reason);
            response
                .widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, false, label));
            let icon_role = match role {
                "command.play" => "transport_play",
                "command.pause" => "transport_pause",
                _ => "transport_step",
            };
            if !crate::resources::paint(
                &transport,
                icon_role,
                role,
                response.id,
                egui::Rect::from_center_size(response.rect.center(), egui::vec2(16.0, 16.0)),
            ) {
                Self::transport_fallback(&transport, role, &response);
            }
            self.workspace.record(role, &response, false);
        }
        self.layout_menu(ui, enabled, rect);
    }

    fn transport_fallback(transport: &egui::Ui, role: &str, response: &egui::Response) {
        let r = response.rect.shrink(6.0);
        let painter = transport.painter();
        if role == "command.pause" {
            for x in [r.left() + 1.0, r.right() - 4.0] {
                painter.rect_filled(
                    egui::Rect::from_min_size(egui::pos2(x, r.top()), egui::vec2(3.0, r.height())),
                    0.0,
                    appearance::MUTED,
                );
            }
        } else {
            painter.add(egui::Shape::convex_polygon(
                vec![
                    r.left_top(),
                    egui::pos2(
                        r.right() - if role == "command.step" { 3.0 } else { 0.0 },
                        r.center().y,
                    ),
                    r.left_bottom(),
                ],
                appearance::MUTED,
                egui::Stroke::NONE,
            ));
            if role == "command.step" {
                painter.line_segment(
                    [r.right_top(), r.right_bottom()],
                    egui::Stroke::new(2.0, appearance::MUTED),
                );
            }
        }
    }

    fn layout_menu(&mut self, ui: &mut egui::Ui, enabled: bool, rect: egui::Rect) {
        let mut right = ui.new_child(
            egui::UiBuilder::new()
                .id_salt("workspace.layout_menu")
                .max_rect(egui::Rect::from_min_max(
                    egui::pos2(rect.right() - 110.0, rect.top()),
                    rect.max,
                ))
                .layout(egui::Layout::right_to_left(egui::Align::Center)),
        );
        right.add_enabled_ui(enabled, |ui| {
            let menu = ui.menu_button("Layout", |ui| {
                if self.menu_item(ui, "Reset Workspace", "command.reset", true) {
                    self.workspace.reset();
                }
            });
            self.workspace.record("menu.layout", &menu.response, false);
        });
    }
}
