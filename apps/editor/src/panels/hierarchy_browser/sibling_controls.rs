use eframe::egui::{self, Rect, Ui};

use super::{BrowserAction, CachedRow};

const BUTTON_WIDTH: f32 = 20.0;

pub(super) fn render(ui: &mut Ui, row: &CachedRow, row_rect: Rect) -> Option<BrowserAction> {
    let (sibling_index, sibling_count) = row.sibling_position?;
    if sibling_count < 2 {
        return None;
    }

    if sibling_index > 0 && first_sibling_clicked(ui, row.id, row_rect) {
        return Some(BrowserAction::SetSiblingIndex(row.id, 0));
    }
    if sibling_index + 1 < sibling_count && last_sibling_clicked(ui, row.id, row_rect) {
        return Some(BrowserAction::SetSiblingIndex(row.id, sibling_count - 1));
    }

    None
}

fn first_sibling_clicked(
    ui: &mut Ui,
    id: openunity_editor_core::scene::EntityId,
    row: Rect,
) -> bool {
    let rect = button_rect(row, 5.0);
    ui.push_id(("first_sibling", id.0), |ui| {
        ui.put(rect, egui::Button::new("↟").frame(false))
            .on_hover_text("Set as first sibling")
            .clicked()
    })
    .inner
}

fn last_sibling_clicked(
    ui: &mut Ui,
    id: openunity_editor_core::scene::EntityId,
    row: Rect,
) -> bool {
    let rect = button_rect(row, 4.0);
    ui.push_id(("last_sibling", id.0), |ui| {
        ui.put(rect, egui::Button::new("↡").frame(false))
            .on_hover_text("Set as last sibling")
            .clicked()
    })
    .inner
}

fn button_rect(row: Rect, distance_from_right: f32) -> Rect {
    Rect::from_min_size(
        egui::pos2(row.right() - BUTTON_WIDTH * distance_from_right, row.top()),
        egui::vec2(BUTTON_WIDTH, row.height()),
    )
}
