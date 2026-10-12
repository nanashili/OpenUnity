//! Toolbar, viewport allocation, pointer navigation and selection requests.

use eframe::egui::{Color32, PointerButton, Rect, Response, Sense, Ui, Vec2};
use openunity_editor_core::{scene::Scene, selection::Selection};

use crate::EditorAction;

use super::ViewportPanel;

const DEFAULT_ZOOM: f64 = 64.0;
const MIN_ZOOM: f64 = 4.0;
const MAX_ZOOM: f64 = 1024.0;
const MAX_CENTER: f64 = 1.0e9;

impl Default for ViewportPanel {
    fn default() -> Self {
        Self {
            center: [0.0; 2],
            zoom: DEFAULT_ZOOM,
            grid_visible: true,
        }
    }
}

impl ViewportPanel {
    /// Apply the independent editor grid preference.
    pub fn set_grid_visible(&mut self, visible: bool) {
        self.grid_visible = visible;
    }

    /// Return the current grid setting, including changes through the toolbar.
    pub fn grid_visible(&self) -> bool {
        self.grid_visible
    }

    /// Draw the current scene and return validated-model selection requests.
    pub fn show(&mut self, ui: &mut Ui, scene: &Scene, selection: &Selection) -> Vec<EditorAction> {
        self.show_toolbar(ui);
        let size = ui.available_size().max(Vec2::splat(1.0));
        let (rect, response) = ui.allocate_exact_size(size, Sense::click_and_drag());
        response.clone().on_hover_text(
            "XY authoring viewport. X points right; Y points up. This is not a Game view.",
        );

        self.navigate(ui, rect, &response);
        let painter = ui.painter_at(rect);
        painter.rect_filled(rect, 0.0, Color32::from_rgb(23, 29, 34));
        self.draw_grid(&painter, rect);

        let hit = self.draw_primitives(&painter, rect, scene, selection, &response);
        if response.clicked_by(PointerButton::Primary)
            && let Some(id) = hit
        {
            return vec![EditorAction::Select {
                id,
                toggle: ui.input(|input| input.modifiers.command || input.modifiers.shift),
            }];
        }
        Vec::new()
    }

    fn show_toolbar(&mut self, ui: &mut Ui) {
        let compact = crate::appearance::is_reference(ui);
        ui.horizontal_wrapped(|ui| {
            ui.strong(if compact {
                "Scene · XY"
            } else {
                "Authoring · Orthographic XY"
            });
            if ui.button("Reset View").clicked() {
                self.center = [0.0; 2];
                self.zoom = DEFAULT_ZOOM;
            }
            ui.checkbox(&mut self.grid_visible, "World grid");
            if compact {
                ui.label("ⓘ").on_hover_text(
                    "Independent primitive preview only. No Game renderer. Drag middle/right to pan; scroll to zoom; click an actual entity to select.",
                );
            }
        });
        if !compact {
            ui.small("Independent primitive preview · drag middle/right to pan · scroll to zoom · click to select");
        }
    }

    fn navigate(&mut self, ui: &Ui, rect: Rect, response: &Response) {
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
                let pointer = pointer.unwrap_or(rect.center());
                let before = self.unproject(rect, pointer);
                self.zoom = (self.zoom * (f64::from(scroll).clamp(-1000.0, 1000.0) * 0.002).exp())
                    .clamp(MIN_ZOOM, MAX_ZOOM);
                let after = self.unproject(rect, pointer);
                self.center[0] += before[0] - after[0];
                self.center[1] += before[1] - after[1];
            }
        }
        for component in &mut self.center {
            *component = component.clamp(-MAX_CENTER, MAX_CENTER);
        }
    }
}
