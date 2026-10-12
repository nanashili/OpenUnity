use eframe::egui::{self, PointerButton, Pos2, Rect, Vec2};
use openunity_editor_core::{
    scene::{Primitive, Scene},
    selection::Selection,
};

use crate::EditorAction;

use super::ViewportPanel;
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
