//! Folder row allocation, semantic response, icon fallback and geometry.

use super::*;

pub(super) fn folder_row(
    ui: &mut egui::Ui,
    label: &str,
    depth: usize,
    selected: bool,
    icon_role: &str,
) -> (egui::Response, serde_json::Value) {
    let row_height = appearance::TREE_LINE_HEIGHT_FALLBACK;
    let (rect, _) = ui.allocate_exact_size(
        egui::vec2(ui.available_width().max(120.0), row_height),
        egui::Sense::hover(),
    );
    let minimum_height = ui.spacing().interact_size.y;
    ui.spacing_mut().interact_size.y = row_height;
    let response = ui.put(rect, egui::Button::new("").selected(selected).frame(false));
    ui.spacing_mut().interact_size.y = minimum_height;
    response.widget_info(|| {
        egui::WidgetInfo::selected(
            egui::WidgetType::SelectableLabel,
            ui.is_enabled(),
            selected,
            label,
        )
    });
    let indent = appearance::tree_indent(depth);
    let x = rect.left() + indent;
    let icon_rect = egui::Rect::from_min_size(
        egui::pos2(x, rect.center().y - appearance::TREE_ICON_WIDTH * 0.5),
        egui::vec2(appearance::TREE_ICON_WIDTH, appearance::TREE_ICON_WIDTH),
    );
    if !crate::resources::paint(ui, icon_role, "project.folder", response.id, icon_rect) {
        appearance::asset_glyph(ui, icon_rect, true, appearance::TEXT);
    }
    let label_origin = egui::pos2(
        icon_rect.right() + appearance::TREE_ICON_LABEL_GAP,
        rect.center().y,
    );
    let label_rect = ui.painter().text(
        label_origin,
        egui::Align2::LEFT_CENTER,
        label,
        egui::FontId::proportional(12.0),
        appearance::TEXT,
    );
    let geometry = serde_json::json!({
        "widget_id": response.id.value(),
        "label": label,
        "depth": depth,
        "row_rect": appearance::rect_json(response.rect),
        "indent_anchor": [x, rect.center().y],
        "indent_without_icon_padding": indent,
        "disclosure_reservation": 0.0,
        "disclosure_rect": null,
        "icon_rect": appearance::rect_json(icon_rect),
        "label_origin": [label_origin.x, label_origin.y],
        "label_rect": appearance::rect_json(label_rect),
        "row_height": response.rect.height(),
        "style_resolved_line_height": null,
        "row_height_source": "fallback"
    });
    (response, geometry)
}
