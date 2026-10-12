//! Independent XY authoring projection of scene primitives, not a game renderer.

use std::collections::{BTreeMap, BTreeSet};

use eframe::egui::{
    self, Align2, Color32, FontId, PointerButton, Pos2, Rect, Sense, Stroke, Ui, Vec2,
};
use openunity_editor_core::scene::{EntityId, MAX_HIERARCHY_DEPTH, Primitive, Scene, Transform};
use openunity_editor_core::selection::Selection;

use crate::EditorAction;

const DEFAULT_ZOOM: f64 = 64.0;
const MIN_ZOOM: f64 = 4.0;
const MAX_ZOOM: f64 = 1024.0;
const MAX_CENTER: f64 = 1.0e9;
const MAX_GRID_LINES: usize = 128;

/// A labelled orthographic XY authoring view with pointer selection and pan/zoom.
///
/// Primitive vertices use local scale, then X/Y/Z Euler rotations in degrees,
/// then translation, composed through the parent hierarchy. This independent
/// convention does not assert Unity Transform or renderer compatibility.
/// Middle/secondary dragging pans; the wheel zooms around the pointer. Only
/// selection requests leave this panel; scene edits remain owned by the shell.
pub struct ViewportPanel {
    center: [f64; 2],
    zoom: f64,
    grid_visible: bool,
}

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
        let compact = crate::appearance::is_reference(ui);
        ui.horizontal_wrapped(|ui| {
            ui.strong(if compact {"Scene · XY"} else {"Authoring · Orthographic XY"});
            if ui.button("Reset View").clicked() {
                self.center = [0.0; 2];
                self.zoom = DEFAULT_ZOOM;
            }
            ui.checkbox(&mut self.grid_visible, "World grid");
            if compact {ui.label("ⓘ").on_hover_text("Independent primitive preview only. No Game renderer. Drag middle/right to pan; scroll to zoom; click an actual entity to select.");}
        });
        if !compact {
            ui.small("Independent primitive preview · drag middle/right to pan · scroll to zoom · click to select");
        }
        let size = ui.available_size().max(Vec2::splat(1.0));
        let (rect, response) = ui.allocate_exact_size(size, Sense::click_and_drag());
        response.clone().on_hover_text(
            "XY authoring viewport. X points right; Y points up. This is not a Game view.",
        );

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
        let painter = ui.painter_at(rect);
        painter.rect_filled(rect, 0.0, Color32::from_rgb(23, 29, 34));
        self.draw_grid(&painter, rect);

        let selected: BTreeSet<_> = selection.ids().collect();
        let mut worlds = BTreeMap::new();
        let mut hit = None;
        let pointer = response.interact_pointer_pos();
        for entity in scene.entities() {
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
            if pointer.is_some_and(|point| bounds.expand(6.0).contains(point)) {
                // Later identities draw above earlier ones, so also win overlap selection.
                hit = Some(entity.id);
            }
        }
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

    fn unproject(&self, rect: Rect, point: Pos2) -> [f64; 2] {
        [
            self.center[0] + f64::from(point.x - rect.center().x) / self.zoom,
            self.center[1] - f64::from(point.y - rect.center().y) / self.zoom,
        ]
    }

    fn project(&self, rect: Rect, point: [f64; 3]) -> Option<Pos2> {
        let x = f64::from(rect.center().x) + (point[0] - self.center[0]) * self.zoom;
        let y = f64::from(rect.center().y) - (point[1] - self.center[1]) * self.zoom;
        let result = Pos2::new(x as f32, y as f32);
        result.is_finite().then_some(result)
    }

    fn draw_grid(&self, painter: &egui::Painter, rect: Rect) {
        if self.grid_visible {
            // Approximately 64 points per grid cell, with a strict line budget.
            let step = 10_f64.powf((64.0 / self.zoom).log10().ceil());
            let low = self.unproject(rect, rect.left_bottom());
            let high = self.unproject(rect, rect.right_top());
            for axis in 0..2 {
                let first = (low[axis] / step).ceil();
                for offset in 0..MAX_GRID_LINES / 2 {
                    let value = (first + offset as f64) * step;
                    if value > high[axis] {
                        break;
                    }
                    let endpoints = if axis == 0 {
                        [[value, low[1], 0.0], [value, high[1], 0.0]]
                    } else {
                        [[low[0], value, 0.0], [high[0], value, 0.0]]
                    };
                    if let (Some(a), Some(b)) = (
                        self.project(rect, endpoints[0]),
                        self.project(rect, endpoints[1]),
                    ) {
                        painter.line_segment([a, b], Stroke::new(1.0, Color32::from_gray(43)));
                    }
                }
            }
        }
        if let Some(origin) = self.project(rect, [0.0; 3]) {
            if rect.y_range().contains(origin.y) {
                painter.line_segment(
                    [
                        Pos2::new(rect.left(), origin.y),
                        Pos2::new(rect.right(), origin.y),
                    ],
                    Stroke::new(1.0, Color32::from_rgb(166, 83, 83)),
                );
                painter.text(
                    Pos2::new(rect.right() - 8.0, origin.y - 4.0),
                    Align2::RIGHT_BOTTOM,
                    "X",
                    FontId::proportional(12.0),
                    Color32::LIGHT_RED,
                );
            }
            if rect.x_range().contains(origin.x) {
                painter.line_segment(
                    [
                        Pos2::new(origin.x, rect.bottom()),
                        Pos2::new(origin.x, rect.top()),
                    ],
                    Stroke::new(1.0, Color32::from_rgb(81, 145, 103)),
                );
                painter.text(
                    Pos2::new(origin.x + 5.0, rect.top() + 6.0),
                    Align2::LEFT_TOP,
                    "Y",
                    FontId::proportional(12.0),
                    Color32::LIGHT_GREEN,
                );
            }
        }
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

#[derive(Clone, Copy)]
struct Affine([[f64; 4]; 3]);

impl Affine {
    fn local(transform: Transform) -> Self {
        let [x, y, z] = transform
            .rotation_deg
            .map(|value| f64::from(value).to_radians());
        let (sx, cx) = x.sin_cos();
        let (sy, cy) = y.sin_cos();
        let (sz, cz) = z.sin_cos();
        let [a, b, c] = transform.scale.map(f64::from);
        Self([
            [
                cz * cy * a,
                (cz * sy * sx - sz * cx) * b,
                (cz * sy * cx + sz * sx) * c,
                f64::from(transform.translation[0]),
            ],
            [
                sz * cy * a,
                (sz * sy * sx + cz * cx) * b,
                (sz * sy * cx - cz * sx) * c,
                f64::from(transform.translation[1]),
            ],
            [
                -sy * a,
                cy * sx * b,
                cy * cx * c,
                f64::from(transform.translation[2]),
            ],
        ])
    }

    fn apply(self, point: [f64; 3]) -> [f64; 3] {
        self.0
            .map(|row| row[0] * point[0] + row[1] * point[1] + row[2] * point[2] + row[3])
    }

    fn compose(self, child: Self) -> Self {
        let mut result = [[0.0; 4]; 3];
        for (row, target) in result.iter_mut().enumerate() {
            for (column, value) in target.iter_mut().enumerate() {
                *value = (0..3)
                    .map(|k| self.0[row][k] * child.0[k][column])
                    .sum::<f64>();
                if column == 3 {
                    *value += self.0[row][3];
                }
            }
        }
        Self(result)
    }
}

fn world_transform(
    scene: &Scene,
    id: EntityId,
    cache: &mut BTreeMap<EntityId, Affine>,
) -> Option<Affine> {
    let mut chain = Vec::new();
    let mut current = id;
    let parent_world = loop {
        if let Some(world) = cache.get(&current) {
            break Some(*world);
        }
        if chain.len() >= MAX_HIERARCHY_DEPTH {
            return None;
        }
        let entity = scene.get(current)?;
        chain.push((current, Affine::local(entity.transform)));
        match entity.parent {
            Some(parent) => current = parent,
            None => break None,
        }
    };
    let mut world = parent_world;
    for (id, local) in chain.into_iter().rev() {
        let next = world.map_or(local, |parent| parent.compose(local));
        if !next.0.iter().flatten().all(|value| value.is_finite()) {
            return None;
        }
        cache.insert(id, next);
        world = Some(next);
    }
    world
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui::{Context, Event, Modifiers, MouseWheelUnit, RawInput};

    fn frame(
        context: &Context,
        viewport: &mut ViewportPanel,
        scene: &Scene,
        events: Vec<Event>,
    ) -> (Vec<EditorAction>, egui::FullOutput) {
        let mut actions = Vec::new();
        let output = context.run(
            RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(800.0, 600.0))),
                events,
                ..Default::default()
            },
            |context| {
                egui::CentralPanel::default().show(context, |ui| {
                    actions = viewport.show(ui, scene, &Selection::default())
                });
            },
        );
        (actions, output)
    }

    fn entity_label(output: &egui::FullOutput, name: &str) -> Pos2 {
        output
            .shapes
            .iter()
            .find_map(|shape| match &shape.shape {
                egui::Shape::Text(text) if text.galley.text() == name => Some(text.pos),
                _ => None,
            })
            .expect("actual entity label must be drawn")
    }

    fn click(
        context: &Context,
        viewport: &mut ViewportPanel,
        scene: &Scene,
        point: Pos2,
    ) -> Vec<EditorAction> {
        frame(
            context,
            viewport,
            scene,
            vec![
                Event::PointerMoved(point),
                Event::PointerButton {
                    pos: point,
                    button: PointerButton::Primary,
                    pressed: true,
                    modifiers: Modifiers::default(),
                },
            ],
        );
        frame(
            context,
            viewport,
            scene,
            vec![Event::PointerButton {
                pos: point,
                button: PointerButton::Primary,
                pressed: false,
                modifiers: Modifiers::default(),
            }],
        )
        .0
    }

    #[test]
    fn headless_viewport_tracks_document_hierarchy_and_pointer_navigation() {
        let context = Context::default();
        let mut viewport = ViewportPanel::default();
        let mut scene = Scene::new("Authoring test").unwrap();
        let parent = scene.create("Parent", Primitive::Empty, None).unwrap();
        let cube = scene
            .create("Actual cube", Primitive::Cube, Some(parent))
            .unwrap();
        scene
            .edit(parent, |entity| {
                entity.transform.translation = [2.0, 1.0, 0.0];
                entity.transform.rotation_deg[2] = 90.0;
            })
            .unwrap();
        scene
            .edit(cube, |entity| {
                entity.transform.translation = [1.0, 0.0, 0.0]
            })
            .unwrap();
        let first = frame(&context, &mut viewport, &scene, vec![]).1;
        let label = entity_label(&first, "Actual cube");
        // Parent's 90-degree rotation places its child one unit above it.
        let parent_label = entity_label(&first, "Parent");
        assert!((label.x - parent_label.x).abs() < 0.01);
        assert!((label.y - parent_label.y + 64.0).abs() < 0.01);
        let point = Pos2::new(label.x - 8.0, label.y + 20.0);
        assert_eq!(
            click(&context, &mut viewport, &scene, point),
            vec![EditorAction::Select {
                id: cube,
                toggle: false
            }]
        );

        scene
            .edit(cube, |entity| entity.transform.translation[0] = 2.0)
            .unwrap();
        let moved = frame(&context, &mut viewport, &scene, vec![]).1;
        let moved_label = entity_label(&moved, "Actual cube");
        assert!((moved_label.y - label.y + 64.0).abs() < 0.01);
        assert!(click(&context, &mut viewport, &scene, point).is_empty());

        let start = Pos2::new(600.0, 400.0);
        frame(
            &context,
            &mut viewport,
            &scene,
            vec![
                Event::PointerMoved(start),
                Event::PointerButton {
                    pos: start,
                    button: PointerButton::Middle,
                    pressed: true,
                    modifiers: Modifiers::default(),
                },
            ],
        );
        let end = start + Vec2::new(40.0, 30.0);
        let panned = frame(
            &context,
            &mut viewport,
            &scene,
            vec![Event::PointerMoved(end)],
        )
        .1;
        let panned_label = entity_label(&panned, "Actual cube");
        assert!((panned_label.x - moved_label.x - 40.0).abs() < 0.01);
        assert!((panned_label.y - moved_label.y - 30.0).abs() < 0.01);
        frame(
            &context,
            &mut viewport,
            &scene,
            vec![Event::PointerButton {
                pos: end,
                button: PointerButton::Middle,
                pressed: false,
                modifiers: Modifiers::default(),
            }],
        );
        let zoomed = frame(
            &context,
            &mut viewport,
            &scene,
            vec![Event::MouseWheel {
                unit: MouseWheelUnit::Point,
                delta: Vec2::new(0.0, 120.0),
                modifiers: Modifiers::default(),
            }],
        )
        .1;
        let zoomed_label = entity_label(&zoomed, "Actual cube");
        assert!(zoomed_label.distance(panned_label) > 5.0);
        for shape in &zoomed.shapes {
            // Transparent toolbar rectangles legitimately have no visual bounds.
            let bounds = shape.shape.visual_bounding_rect();
            assert!(
                bounds == Rect::NOTHING || bounds.is_finite(),
                "nonfinite shape: {:?}",
                shape.shape
            );
        }
    }
}
