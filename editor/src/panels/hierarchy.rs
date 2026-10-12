//! Scene hierarchy controls for independent OpenUnity authoring.

use std::collections::{BTreeMap, BTreeSet};

use eframe::egui::{ComboBox, ScrollArea, Ui};
use openunity_editor_core::scene::{Entity, EntityId, MAX_HIERARCHY_DEPTH, Primitive, Scene};
use openunity_editor_core::selection::Selection;

use super::hierarchy_browser::{HierarchyBrowser, Row};
use crate::EditorAction;

/// Compact reference presentation over the same actual forest and actions.
pub fn show_reference(
    ui: &mut Ui,
    scene: &Scene,
    selection: &Selection,
    browser: &mut HierarchyBrowser,
) -> Vec<EditorAction> {
    let mut actions = Vec::new();
    let primary = selection.primary().filter(|id| scene.get(*id).is_some());
    ui.horizontal(|ui| {
        ui.menu_button("+", |ui| {
            for (label, primitive) in [
                ("Add Empty", Primitive::Empty),
                ("Add Cube", Primitive::Cube),
                ("Add Camera", Primitive::Camera),
            ] {
                if ui.button(label).clicked() {
                    actions.push(EditorAction::Create {
                        primitive,
                        parent: primary,
                    });
                    ui.close();
                }
            }
            if ui
                .add_enabled(
                    primary.is_some(),
                    eframe::egui::Button::new("Delete selection"),
                )
                .clicked()
            {
                actions.push(EditorAction::DeleteSelection);
                ui.close();
            }
            if let Some(entity) = primary.and_then(|id| scene.get(id)) {
                let mut parent = entity.parent;
                ComboBox::from_id_salt(("hierarchy_parent", entity.id.0))
                    .selected_text("Parent")
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut parent, None, "Scene root");
                        for candidate in scene.entities().filter(|e| e.id != entity.id) {
                            ui.selectable_value(&mut parent, Some(candidate.id), &candidate.name);
                        }
                    });
                if parent != entity.parent {
                    actions.push(EditorAction::Reparent {
                        id: entity.id,
                        parent,
                    });
                }
            }
        });
        browser.search(ui);
    });
    let mut children: BTreeMap<Option<EntityId>, Vec<&Entity>> = BTreeMap::new();
    for entity in scene.entities() {
        children.entry(entity.parent).or_default().push(entity);
    }
    let mut pending = Vec::new();
    if let Some(roots) = children.get(&None) {
        pending.extend(roots.iter().rev().map(|e| (*e, 0)));
    }
    let mut rows = Vec::new();
    while let Some((entity, depth)) = pending.pop() {
        rows.push(Row {
            id: entity.id,
            parent: entity.parent,
            name: entity.name.clone(),
            depth,
            active: entity.active,
            selected: selection.ids().any(|id| id == entity.id),
        });
        if depth + 1 < MAX_HIERARCHY_DEPTH
            && let Some(descendants) = children.get(&Some(entity.id))
        {
            pending.extend(descendants.iter().rev().map(|e| (*e, depth + 1)));
        }
    }
    actions.extend(
        browser
            .tree(ui, scene.name(), &rows)
            .into_iter()
            .map(|(id, toggle)| EditorAction::Select { id, toggle }),
    );
    actions
}

/// Display the live forest and return document operations for the shell to validate.
pub fn show(ui: &mut Ui, scene: &Scene, selection: &Selection) -> Vec<EditorAction> {
    let mut actions = Vec::new();
    let primary = selection.primary().filter(|id| scene.get(*id).is_some());
    let selected: BTreeSet<_> = selection.ids().collect();

    ui.horizontal_wrapped(|ui| {
        for (label, primitive) in [
            ("Add Empty", Primitive::Empty),
            ("Add Cube", Primitive::Cube),
            ("Add Camera", Primitive::Camera),
        ] {
            if ui.button(label).clicked() {
                actions.push(EditorAction::Create {
                    primitive,
                    parent: primary,
                });
            }
        }
        if ui
            .add_enabled(
                selected.iter().any(|id| scene.get(*id).is_some()),
                eframe::egui::Button::new("Delete selection"),
            )
            .clicked()
        {
            actions.push(EditorAction::DeleteSelection);
        }
    });
    ui.small("Rename the selected entity in Inspector.");

    if let Some(entity) = primary.and_then(|id| scene.get(id)) {
        let mut parent = entity.parent;
        ui.horizontal(|ui| {
            ui.label("Parent");
            ComboBox::from_id_salt(("hierarchy_parent", entity.id.0))
                .selected_text(parent.and_then(|id| scene.get(id)).map_or_else(
                    || "Scene root".to_owned(),
                    |entity| format!("{} ({})", entity.name, entity.id.0),
                ))
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut parent, None, "Scene root");
                    for candidate in scene
                        .entities()
                        .filter(|candidate| candidate.id != entity.id)
                    {
                        ui.selectable_value(
                            &mut parent,
                            Some(candidate.id),
                            format!("{} ({})", candidate.name, candidate.id.0),
                        );
                    }
                });
        });
        if parent != entity.parent {
            actions.push(EditorAction::Reparent {
                id: entity.id,
                parent,
            });
        }
    }
    ui.separator();

    // Index once: scanning the full document for every row is quadratic.
    // Scene validation guarantees an acyclic forest with bounded depth.
    let mut children: BTreeMap<Option<EntityId>, Vec<&Entity>> = BTreeMap::new();
    for entity in scene.entities() {
        children.entry(entity.parent).or_default().push(entity);
    }
    ScrollArea::both()
        .id_salt("hierarchy_forest")
        .show(ui, |ui| {
            if scene.entities().len() == 0 {
                ui.label("Scene is empty. Add an entity to begin.");
            }
            let mut pending = Vec::new();
            if let Some(roots) = children.get(&None) {
                pending.extend(roots.iter().rev().map(|entity| (*entity, 0)));
            }
            while let Some((entity, depth)) = pending.pop() {
                ui.push_id(entity.id.0, |ui| {
                    ui.horizontal(|ui| {
                        ui.add_space(depth as f32 * 14.0);
                        let active = if entity.active { "active" } else { "inactive" };
                        let response = ui.selectable_label(
                            selected.contains(&entity.id),
                            format!("{} ({active})", entity.name),
                        );
                        if response.clicked() {
                            let toggle =
                                ui.input(|input| input.modifiers.command || input.modifiers.ctrl);
                            actions.push(EditorAction::Select {
                                id: entity.id,
                                toggle,
                            });
                        }
                    });
                });
                if depth + 1 < MAX_HIERARCHY_DEPTH
                    && let Some(descendants) = children.get(&Some(entity.id))
                {
                    pending.extend(descendants.iter().rev().map(|child| (*child, depth + 1)));
                }
            }
        });
    actions
}

#[cfg(test)]
mod tests {
    use eframe::egui::{
        CentralPanel, Context, Event, FullOutput, Modifiers, PointerButton, Pos2, RawInput, Rect,
        Vec2, epaint::Shape,
    };

    use super::*;

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
}
