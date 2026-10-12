//! Pointer navigation over disposable viewport state.

use super::ViewportPanel;
use eframe::egui::{PointerButton, Rect, Response, Ui};

impl ViewportPanel {
    pub(super) fn navigate(&mut self, ui: &Ui, rect: Rect, response: &Response) {
        if response.dragged_by(PointerButton::Middle)
            || response.dragged_by(PointerButton::Secondary)
        {
            let delta = response.drag_delta();
            if delta.is_finite() {
                self.center[0] -= f64::from(delta.x) / self.zoom;
                self.center[1] += f64::from(delta.y) / self.zoom;
            }
        }
        if response.hovered() {
            let (scroll, pointer) =
                ui.input(|input| (input.raw_scroll_delta.y, input.pointer.hover_pos()));
            if scroll.is_finite() && scroll != 0.0 {
                let point = pointer.unwrap_or(rect.center());
                let before = self.unproject(rect, point);
                self.zoom = (self.zoom * (f64::from(scroll).clamp(-1000.0, 1000.0) * 0.002).exp())
                    .clamp(4.0, 1024.0);
                let after = self.unproject(rect, point);
                self.center[0] += before[0] - after[0];
                self.center[1] += before[1] - after[1];
            }
        }
        for coordinate in &mut self.center {
            *coordinate = coordinate.clamp(-1.0e9, 1.0e9);
        }
    }
}
