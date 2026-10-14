//! Primitive wire geometry, ordered drawing and overlap hit selection.

use std::collections::{BTreeMap, BTreeSet};

use eframe::egui::{self, Align2, Color32, FontId, Rect, Stroke, Vec2};
use openunity_editor_core::{
    scene::{EntityId, Primitive, Scene},
    selection::Selection,
};

use super::{ViewportPanel, transforms::world_transform};

impl ViewportPanel {
    pub(super) fn draw_primitives(
        &self,
        painter: &egui::Painter,
        rect: Rect,
        scene: &Scene,
        selection: &Selection,
        visibility: &crate::scene_visibility::SceneVisibility,
        response: &egui::Response,
    ) -> Option<EntityId> {
        let selected: BTreeSet<_> = selection.ids().collect();
        let mut worlds = BTreeMap::new();
        let mut hit = None;
        let pointer = response.interact_pointer_pos();
        for entity in scene.entities() {
            if !visibility.is_visible(entity.id) {
                continue;
            }
            let Some(world) = world_transform(scene, entity.id, &mut worlds) else {
                continue;
            };
            let (vertices, edges) = primitive_geometry(entity.primitive);
            let points: Option<Vec<_>> = vertices
                .iter()
                .map(|point| self.project(rect, world.apply(*point)))
                .collect();
            let Some(points) = points else {
                continue;
            };
            let bounds = Rect::from_points(&points);
            if !bounds.expand(12.0).intersects(rect) {
                continue;
            }
            let is_selected = selected.contains(&entity.id);
            let color = if is_selected {
                Color32::from_rgb(255, 199, 92)
            } else if entity.active {
                Color32::from_rgb(133, 192, 219)
            } else {
                Color32::from_gray(100)
            };
            for &(a, b) in edges {
                painter.line_segment(
                    [points[a], points[b]],
                    Stroke::new(if is_selected { 2.5 } else { 1.5 }, color),
                );
            }
            let center = self.project(rect, world.apply([0.0; 3]));
            if let Some(center) = center {
                // Keep zero-scale primitives visible and selectable as a point.
                painter.circle_filled(center, if is_selected { 3.0 } else { 2.0 }, color);
                painter.text(
                    center + Vec2::new(8.0, -8.0),
                    Align2::LEFT_BOTTOM,
                    &entity.name,
                    FontId::proportional(12.0),
                    color,
                );
            }
            if is_selected {
                painter.rect_stroke(
                    bounds.expand(5.0),
                    2.0,
                    Stroke::new(1.0, color),
                    egui::StrokeKind::Outside,
                );
            }
            if visibility.is_pickable(entity.id)
                && pointer.is_some_and(|point| bounds.expand(6.0).contains(point))
            {
                // Later identities draw above earlier ones, so also win overlap selection.
                hit = Some(entity.id);
            }
        }
        hit
    }
}

// Small independently authored wire representations in local coordinates.
fn primitive_geometry(primitive: Primitive) -> (&'static [[f64; 3]], &'static [(usize, usize)]) {
    match primitive {
        Primitive::Cube => (
            &[
                [-0.5, -0.5, -0.5],
                [0.5, -0.5, -0.5],
                [0.5, 0.5, -0.5],
                [-0.5, 0.5, -0.5],
                [-0.5, -0.5, 0.5],
                [0.5, -0.5, 0.5],
                [0.5, 0.5, 0.5],
                [-0.5, 0.5, 0.5],
            ],
            &[
                (0, 1),
                (1, 2),
                (2, 3),
                (3, 0),
                (4, 5),
                (5, 6),
                (6, 7),
                (7, 4),
                (0, 4),
                (1, 5),
                (2, 6),
                (3, 7),
            ],
        ),
        Primitive::Empty => (
            &[
                [-0.2, 0.0, 0.0],
                [0.0, 0.2, 0.0],
                [0.2, 0.0, 0.0],
                [0.0, -0.2, 0.0],
            ],
            &[(0, 1), (1, 2), (2, 3), (3, 0), (0, 2), (1, 3)],
        ),
        Primitive::Camera => (
            &[
                [-0.3, -0.2, 0.0],
                [0.2, -0.2, 0.0],
                [0.2, 0.2, 0.0],
                [-0.3, 0.2, 0.0],
                [0.6, -0.4, 0.0],
                [0.6, 0.4, 0.0],
            ],
            &[(0, 1), (1, 2), (2, 3), (3, 0), (1, 4), (4, 5), (5, 2)],
        ),
    }
}
