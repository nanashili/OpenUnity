//! Decorative severity icons paired with their existing checkbox controls.

use super::*;

impl ConsolePanel {
    pub(super) fn severity_checkbox(
        ui: &mut Ui,
        checked: &mut bool,
        label: &str,
        resource_role: &str,
        retained_count: usize,
    ) -> (egui::Response, serde_json::Value) {
        let galley = egui::WidgetText::from(label).into_galley(
            ui,
            Some(egui::TextWrapMode::Extend),
            f32::INFINITY,
            egui::FontSelection::Default,
        );
        let checkbox_width =
            (ui.spacing().icon_width + ui.spacing().icon_spacing + galley.size().x)
                .max(ui.spacing().interact_size.y);
        // Wrap the entire decorative pair without creating a child UI or another
        // widget ID. The checkbox remains the existing semantic/focus control.
        if ui.available_rect_before_wrap().width() < checkbox_width + 18.0
            && ui.cursor().left() > ui.max_rect().left()
        {
            ui.end_row();
        }
        let response = ui.checkbox(checked, label);
        let icon_rect = egui::Rect::from_min_size(
            egui::pos2(response.rect.right() + 2.0, response.rect.center().y - 8.0),
            egui::vec2(16.0, 16.0),
        );
        ui.add_space(18.0);
        let role = format!("console.severity.{label}");
        let painted = crate::resources::paint_counted(
            ui,
            resource_role,
            &role,
            response.id,
            icon_rect,
            retained_count,
        );
        let geometry = Self::severity_geometry(
            &response,
            &role,
            label,
            resource_role,
            retained_count,
            icon_rect,
            painted,
        );
        (response, geometry)
    }

    fn severity_geometry(
        response: &egui::Response,
        role: &str,
        label: &str,
        resource_role: &str,
        retained_count: usize,
        icon_rect: egui::Rect,
        painted: bool,
    ) -> serde_json::Value {
        serde_json::json!({
            "role": role,
            "widget_id": response.id.value(),
            "severity": label,
            "resource_role": resource_role,
            "retained_count": retained_count,
            "presentation_state": if retained_count == 0 {
                "zero_count"
            } else {
                "positive_count"
            },
            "checkbox_rect": crate::appearance::rect_json(response.rect),
            "icon_rect": crate::appearance::rect_json(icon_rect),
            "group_rect": crate::appearance::rect_json(response.rect.union(icon_rect)),
            "paint_success": painted,
        })
    }
}
