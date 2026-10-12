use super::*;
use eframe::egui;
use egui::{Context, Event, FullOutput, PointerButton, Pos2, RawInput, Rect, Vec2};
use openunity_editor_core::scene::Primitive;

fn frame(
    context: &Context,
    panel: &mut InspectorPanel,
    scene: &Scene,
    selection: &Selection,
    events: Vec<Event>,
) -> (Vec<EditorAction>, FullOutput) {
    let mut actions = Vec::new();
    let output = context.run(
        RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(600.0, 600.0))),
            events,
            ..Default::default()
        },
        |context| {
            egui::CentralPanel::default().show(context, |ui| {
                actions = panel.show(ui, scene, selection);
            });
        },
    );
    (actions, output)
}

fn text_position(output: &FullOutput, text: &str) -> Pos2 {
    output
        .shapes
        .iter()
        .find_map(|shape| match &shape.shape {
            egui::epaint::Shape::Text(shape) if shape.galley.job.text == text => {
                Some(shape.pos + shape.galley.size() * 0.5)
            }
            _ => None,
        })
        .unwrap_or_else(|| panic!("missing visible text: {text}"))
}

fn click(
    context: &Context,
    panel: &mut InspectorPanel,
    scene: &Scene,
    selection: &Selection,
    position: Pos2,
) -> Vec<EditorAction> {
    frame(
        context,
        panel,
        scene,
        selection,
        vec![
            Event::PointerMoved(position),
            Event::PointerButton {
                pos: position,
                button: PointerButton::Primary,
                pressed: true,
                modifiers: Default::default(),
            },
        ],
    );
    frame(
        context,
        panel,
        scene,
        selection,
        vec![Event::PointerButton {
            pos: position,
            button: PointerButton::Primary,
            pressed: false,
            modifiers: Default::default(),
        }],
    )
    .0
}

#[test]
fn apply_emits_one_complete_edit_without_mutating_the_scene() {
    let context = Context::default();
    let mut panel = InspectorPanel::default();
    let mut scene = Scene::new("Inspector input").unwrap();
    let id = scene.create("Before", Primitive::Cube, None).unwrap();
    let before = scene.clone();
    let mut selection = Selection::new();
    selection.replace(Some(id), &scene).unwrap();
    frame(&context, &mut panel, &scene, &selection, Vec::new());

    // Author a draft, then exercise the actual Apply widget through pointer input.
    let draft = panel.draft.as_mut().unwrap();
    draft.name = "After".into();
    draft.active = false;
    draft.transform = Transform {
        translation: [1.0, 2.0, 3.0],
        rotation_deg: [10.0, 20.0, 30.0],
        scale: [-1.0, 0.0, 2.0],
    };
    let expected = EditorAction::Edit {
        id,
        name: "After".into(),
        active: false,
        transform: draft.transform,
    };
    let (actions, output) = frame(&context, &mut panel, &scene, &selection, Vec::new());
    assert!(actions.is_empty());
    let position = text_position(&output, "Apply");
    assert_eq!(
        click(&context, &mut panel, &scene, &selection, position),
        [expected]
    );
    assert_eq!(scene, before);

    // Invalid drafts remain visible and cannot produce actions from Apply.
    for invalid in ["empty", "long", "nonfinite"] {
        let draft = panel.draft.as_mut().unwrap();
        draft.name = "After".into();
        draft.transform = Transform::default();
        let error = match invalid {
            "empty" => {
                draft.name.clear();
                "Edit entity: name must not be empty."
            }
            "long" => {
                draft.name = "é".repeat(MAX_NAME_BYTES / 2 + 1);
                "Edit entity: name exceeds 1024 UTF-8 bytes."
            }
            _ => {
                draft.transform.scale[2] = f32::INFINITY;
                "Edit entity: all transform values must be finite."
            }
        };
        let (_, output) = frame(&context, &mut panel, &scene, &selection, Vec::new());
        text_position(&output, error);
        assert!(
            click(
                &context,
                &mut panel,
                &scene,
                &selection,
                text_position(&output, "Apply"),
            )
            .is_empty()
        );
        assert_eq!(scene, before);
    }
}

#[test]
fn selection_and_external_edits_refresh_drafts_and_empty_selection_clears_form() {
    let context = Context::default();
    let mut panel = InspectorPanel::default();
    let mut scene = Scene::new("Inspector lifecycle").unwrap();
    let first = scene.create("First", Primitive::Empty, None).unwrap();
    let second = scene
        .create("Second", Primitive::Camera, Some(first))
        .unwrap();
    let mut selection = Selection::new();
    let (_, output) = frame(&context, &mut panel, &scene, &selection, Vec::new());
    text_position(&output, "Select an entity to inspect its local properties.");

    selection.replace(Some(second), &scene).unwrap();
    let (_, output) = frame(&context, &mut panel, &scene, &selection, Vec::new());
    text_position(&output, "Primitive: Camera");
    text_position(&output, &format!("Parent: {}", first.0));
    panel.draft.as_mut().unwrap().name = "Uncommitted".into();
    scene
        .edit(second, |entity| entity.name = "Restored".into())
        .unwrap();
    frame(&context, &mut panel, &scene, &selection, Vec::new());
    assert_eq!(panel.draft.as_ref().unwrap().name, "Restored");

    selection.toggle(first, &scene).unwrap();
    let (_, output) = frame(&context, &mut panel, &scene, &selection, Vec::new());
    text_position(
        &output,
        "2 entities selected. Editing only the primary entity.",
    );
    assert_eq!(panel.draft.as_ref().unwrap().name, "First");
    selection.clear();
    let (actions, output) = frame(&context, &mut panel, &scene, &selection, Vec::new());
    assert!(actions.is_empty());
    text_position(&output, "Select an entity to inspect its local properties.");
    assert!(panel.draft.is_none());
}
