//! Resource overlays on the existing semantic search and disclosure controls.

use eframe::egui;

use super::painting::paint;

/// Overlay a reviewed resource on the unchanged semantic clear-search button.
pub(crate) fn search_cancel_button(
    ui: &mut egui::Ui,
    enabled: bool,
    widget_role: &str,
) -> egui::Response {
    if !crate::appearance::is_reference(ui) {
        return ui.add_enabled(enabled, egui::Button::new("×"));
    }
    ui.add_enabled(enabled, |ui: &mut egui::Ui| {
        let response = ui.add(egui::Button::new(
            egui::RichText::new("×").color(egui::Color32::TRANSPARENT),
        ));
        let role = if !enabled {
            "search_cancel_off"
        } else if response.is_pointer_button_down_on() {
            "search_cancel_active"
        } else {
            "search_cancel"
        };
        let rect = egui::Rect::from_center_size(response.rect.center(), egui::vec2(14.0, 14.0));
        let painted = response.rect.contains_rect(rect)
            && ui.clip_rect().contains_rect(rect)
            && paint(ui, role, widget_role, response.id, rect);
        if !painted {
            ui.painter().text(
                response.rect.center(),
                egui::Align2::CENTER_CENTER,
                "×",
                egui::TextStyle::Button.resolve(ui.style()),
                ui.style().interact(&response).text_color(),
            );
        }
        response
    })
}

pub(crate) fn disclosure(ui: &mut egui::Ui, openness: f32, response: &egui::Response) {
    let role = if openness > 0.5 {
        "disclosure_open"
    } else {
        "disclosure_closed"
    };
    let rect = egui::Rect::from_center_size(response.rect.center(), egui::vec2(13.0, 13.0));
    if !paint(ui, role, "inspector.disclosure", response.id, rect) {
        egui::collapsing_header::paint_default_icon(ui, openness, response);
    }
}
