//! Viewport control order, derived paint and selection requests.

use eframe::egui::{Color32, PointerButton, Sense, Ui, Vec2};
use openunity_editor_core::imported_scene::ImportedDocument;

use super::super::{ImportedAction, actions::selection_toggle};
use super::ViewportPanel;
use crate::appearance;

impl ViewportPanel {
    /// Draw a disposable derived preview without modifying any serialized value.
    pub fn show(&mut self, ui: &mut Ui, document: &ImportedDocument) -> Vec<ImportedAction> {
        self.controls(ui);
        let (rect, response) = ui.allocate_exact_size(
            ui.available_size().max(Vec2::splat(1.0)),
            Sense::click_and_drag(),
        );
        response.clone().on_hover_text(
            "Independent orthographic XY Transform preview; no Unity renderer equivalence.",
        );
        self.navigate(ui, rect, &response);
        let painter = ui.painter_at(rect);
        painter.rect_filled(rect, 0.0, Color32::from_rgb(23, 29, 34));
        self.grid(rect, &painter);
        let (hit, omitted, view_error) = self.markers(document, rect, &response, &painter);
        Self::diagnostics(rect, &painter, omitted, view_error);
        if response.clicked_by(PointerButton::Primary) {
            return vec![ImportedAction::Select {
                id: hit,
                toggle: selection_toggle(ui),
            }];
        }
        Vec::new()
    }

    fn controls(&mut self, ui: &mut Ui) {
        let compact = appearance::is_reference(ui);
        ui.horizontal_wrapped(|ui| {
            ui.strong(if compact {
                "Scene · Transform XY"
            } else {
                "Imported Transform preview · XY"
            });
            if ui.button("Reset View").clicked() {
                self.center = [0.0; 2];
                self.zoom = 64.0;
            }
            ui.checkbox(&mut self.grid_visible, "World grid");
            if compact {
                ui.label("ⓘ").on_hover_text(
                    "Read-only Transform markers and axes only; no meshes, cameras, materials or Game rendering. Independent quaternion-derived preview. Middle/right drag to pan; scroll to zoom; click to select.",
                );
            }
        });
        if !compact {
            ui.small(
                "Read-only markers and axes · no meshes, cameras, materials or Game rendering.",
            );
            ui.small("Independent quaternion-derived preview · middle/right drag to pan · scroll to zoom · click to select.");
        }
    }
}
