//! Bounded grid and authored-order world marker painting.

use super::{Affine, ViewportPanel};
use eframe::egui::{Align2, Color32, FontId, Painter, Pos2, Rect, Response, Stroke, Vec2};
use openunity_editor_core::imported_scene::ImportedDocument;
use openunity_editor_core::scene::EntityId;

impl ViewportPanel {
    pub(super) fn grid(&self, rect: Rect, painter: &Painter) {
        if self.grid_visible {
            for axis in 0..2 {
                let half = f64::from(if axis == 0 {
                    rect.width()
                } else {
                    rect.height()
                }) / (2.0 * self.zoom);
                let start = (self.center[axis] - half).floor();
                let end = (self.center[axis] + half).ceil();
                // Bounded grid density, independent of scene magnitudes.
                let step = ((end - start) / 128.0).ceil().max(1.0);
                for index in 0..=128 {
                    let coordinate = start + f64::from(index) * step;
                    if coordinate > end {
                        break;
                    }
                    let mut point = self.center;
                    point[axis] = coordinate;
                    if let Some(point) = self.project(rect, [point[0], point[1], 0.0]) {
                        let segment = if axis == 0 {
                            [
                                Pos2::new(point.x, rect.top()),
                                Pos2::new(point.x, rect.bottom()),
                            ]
                        } else {
                            [
                                Pos2::new(rect.left(), point.y),
                                Pos2::new(rect.right(), point.y),
                            ]
                        };
                        painter.line_segment(segment, Stroke::new(1.0, Color32::from_gray(43)));
                    }
                }
            }
        }
    }

    pub(super) fn markers(
        &self,
        document: &ImportedDocument,
        rect: Rect,
        response: &Response,
        painter: &Painter,
    ) -> (Option<EntityId>, usize, Option<String>) {
        let mut pending: Vec<_> = document
            .roots()
            .into_iter()
            .rev()
            .map(|id| (id, Some(Affine::IDENTITY)))
            .collect();
        let mut omitted = 0;
        let mut view_error = None;
        let mut hit = None;
        while let Some((id, parent)) = pending.pop() {
            let entity = match document.entity(id) {
                Ok(entity) => entity,
                Err(error) => {
                    view_error.get_or_insert_with(|| error.to_string());
                    omitted += 1;
                    continue;
                }
            };
            let world = parent.and_then(|parent| {
                parent.compose(Affine::local(
                    entity.transform.local_position,
                    entity.transform.local_rotation,
                    entity.transform.local_scale,
                ))
            });
            match document.children(id) {
                Ok(children) => pending.extend(children.into_iter().rev().map(|id| (id, world))),
                Err(error) => {
                    view_error.get_or_insert_with(|| error.to_string());
                }
            }
            let Some(world) = world else {
                omitted += 1;
                continue;
            };
            let Some(origin) = self.project(rect, world.apply([0.0; 3])) else {
                omitted += 1;
                continue;
            };
            if !rect.expand(32.0).contains(origin) {
                continue;
            }
            let selected = document.selection().contains(&id);
            let color = if selected {
                Color32::from_rgb(255, 199, 92)
            } else if entity.game_object.active {
                Color32::from_rgb(133, 192, 219)
            } else {
                Color32::from_gray(110)
            };
            for (index, axis_color) in [
                Color32::LIGHT_RED,
                Color32::LIGHT_GREEN,
                Color32::LIGHT_BLUE,
            ]
            .into_iter()
            .enumerate()
            {
                let mut endpoint = [0.0; 3];
                endpoint[index] = 0.5;
                if let Some(end) = self.project(rect, world.apply(endpoint)) {
                    painter.line_segment(
                        [origin, end],
                        Stroke::new(if selected { 2.0 } else { 1.0 }, axis_color),
                    );
                    painter.text(
                        end + Vec2::new(3.0, index as f32 * 11.0),
                        Align2::LEFT_TOP,
                        ["X", "Y", "Z"][index],
                        FontId::proportional(10.0),
                        axis_color,
                    );
                } else {
                    omitted += 1;
                }
            }
            painter.circle_filled(origin, if selected { 5.0 } else { 3.5 }, color);
            painter.text(
                origin + Vec2::new(8.0, -8.0),
                Align2::LEFT_BOTTOM,
                format!(
                    "{} · fileID {}",
                    entity.game_object.name, entity.game_object.file_id
                ),
                FontId::proportional(12.0),
                color,
            );
            if response
                .interact_pointer_pos()
                .is_some_and(|point| point.distance(origin) <= 12.0)
            {
                hit = Some(id);
            }
        }
        (hit, omitted, view_error)
    }

    pub(super) fn diagnostics(
        rect: Rect,
        painter: &Painter,
        omitted: usize,
        view_error: Option<String>,
    ) {
        if omitted > 0 {
            painter.text(rect.left_top() + Vec2::splat(8.0), Align2::LEFT_TOP,
                format!("{omitted} Transform markers/axes omitted: preview range or unavailable view. Hierarchy/Inspector remain available."),
                FontId::proportional(12.0), Color32::LIGHT_RED);
        }
        if let Some(error) = view_error {
            painter.text(
                rect.left_top() + Vec2::new(8.0, 26.0),
                Align2::LEFT_TOP,
                error,
                FontId::proportional(12.0),
                Color32::LIGHT_RED,
            );
        }
    }
}
