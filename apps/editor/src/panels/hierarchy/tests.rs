use eframe::egui::{
    CentralPanel, Context, Event, FullOutput, Modifiers, PointerButton, Pos2, RawInput, Rect, Vec2,
    epaint::Shape,
};

use super::*;
use crate::EditorAction;
use openunity_editor_core::scene::{Primitive, Scene};
use openunity_editor_core::selection::Selection;

fn frame(
    ctx: &Context,
    scene: &Scene,
    selection: &Selection,
    events: Vec<Event>,
    modifiers: Modifiers,
) -> (FullOutput, Vec<EditorAction>) {
    let mut actions = Vec::new();
    let output = ctx.run(
        RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(800.0, 600.0))),
            events,
            modifiers,
            ..Default::default()
        },
        |ctx| {
            CentralPanel::default().show(ctx, |ui| actions = show(ui, scene, selection));
        },
    );
    (output, actions)
}

fn text_position(output: &FullOutput, label: &str) -> Pos2 {
    output
        .shapes
        .iter()
        .find_map(|shape| {
            if let Shape::Text(text) = &shape.shape {
                (text.galley.text() == label).then(|| text.pos + text.galley.size() * 0.5)
            } else {
                None
            }
        })
        .unwrap_or_else(|| panic!("missing visible text: {label}"))
}

fn click(
    ctx: &Context,
    scene: &Scene,
    selection: &Selection,
    pos: Pos2,
    modifiers: Modifiers,
) -> Vec<EditorAction> {
    frame(
        ctx,
        scene,
        selection,
        vec![
            Event::PointerMoved(pos),
            Event::PointerButton {
                pos,
                button: PointerButton::Primary,
                pressed: true,
                modifiers,
            },
            Event::PointerButton {
                pos,
                button: PointerButton::Primary,
                pressed: false,
                modifiers,
            },
        ],
        modifiers,
    )
    .1
}

#[test]
fn forest_rows_follow_real_parents_and_pointer_selection_preserves_identity() {
    let mut scene = Scene::new("Hierarchy test").unwrap();
    let root = scene.create("First root", Primitive::Empty, None).unwrap();
    let other = scene
        .create("Second root", Primitive::Camera, None)
        .unwrap();
    let child = scene
        .create("Nested cube", Primitive::Cube, Some(root))
        .unwrap();
    scene.edit(child, |entity| entity.active = false).unwrap();
    let mut selection = Selection::new();
    selection.replace(Some(root), &scene).unwrap();
    let ctx = Context::default();
    let (output, actions) = frame(&ctx, &scene, &selection, Vec::new(), Modifiers::NONE);
    assert!(actions.is_empty());
    let root_pos = text_position(&output, "First root (active)");
    let child_pos = text_position(&output, "Nested cube (inactive)");
    let other_pos = text_position(&output, "Second root (active)");
    assert!(root_pos.y < child_pos.y && child_pos.y < other_pos.y);
    assert_eq!(
        click(&ctx, &scene, &selection, child_pos, Modifiers::NONE),
        [EditorAction::Select {
            id: child,
            toggle: false
        }]
    );
    let modifiers = Modifiers {
        ctrl: true,
        command: true,
        ..Modifiers::NONE
    };
    assert_eq!(
        click(&ctx, &scene, &selection, other_pos, modifiers),
        [EditorAction::Select {
            id: other,
            toggle: true
        }]
    );
    assert_eq!(selection.primary(), Some(root));
}

#[test]
fn creation_uses_primary_parent_and_deletion_requires_live_selection() {
    let mut scene = Scene::new("Controls test").unwrap();
    let root = scene.create("Parent", Primitive::Empty, None).unwrap();
    let mut selection = Selection::new();
    let ctx = Context::default();
    let (output, _) = frame(&ctx, &scene, &selection, Vec::new(), Modifiers::NONE);
    let delete = text_position(&output, "Delete selection");
    assert!(click(&ctx, &scene, &selection, delete, Modifiers::NONE).is_empty());
    selection.replace(Some(root), &scene).unwrap();
    let (output, _) = frame(&ctx, &scene, &selection, Vec::new(), Modifiers::NONE);
    assert_eq!(
        click(
            &ctx,
            &scene,
            &selection,
            text_position(&output, "Add Cube"),
            Modifiers::NONE
        ),
        [EditorAction::Create {
            primitive: Primitive::Cube,
            parent: Some(root)
        }]
    );
    assert_eq!(
        click(
            &ctx,
            &scene,
            &selection,
            text_position(&output, "Delete selection"),
            Modifiers::NONE
        ),
        [EditorAction::DeleteSelection]
    );
}
