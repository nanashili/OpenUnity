//! Independent authoring controls and selectable forest rows.

use std::collections::{BTreeMap, BTreeSet};

use eframe::egui::{ComboBox, ScrollArea, Ui};
use openunity_editor_core::scene::{Entity, EntityId, MAX_HIERARCHY_DEPTH, Primitive, Scene};
use openunity_editor_core::selection::Selection;

use crate::EditorAction;

/// Display the live forest and return document operations for the shell to validate.
pub fn show(ui: &mut Ui, scene: &Scene, selection: &Selection) -> Vec<EditorAction> {
    let mut actions = Vec::new();
    let primary = selection.primary().filter(|id| scene.get(*id).is_some());
    let selected: BTreeSet<_> = selection.ids().collect();

    creation_controls(ui, scene, primary, &selected, &mut actions);
    parent_control(ui, scene, primary, &mut actions);
    forest(ui, scene, &selected, &mut actions);
    actions
}

fn creation_controls(
    ui: &mut Ui,
    scene: &Scene,
    primary: Option<EntityId>,
    selected: &BTreeSet<EntityId>,
    actions: &mut Vec<EditorAction>,
) {
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
}

fn parent_control(
    ui: &mut Ui,
    scene: &Scene,
    primary: Option<EntityId>,
    actions: &mut Vec<EditorAction>,
) {
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
}

fn forest(
    ui: &mut Ui,
    scene: &Scene,
    selected: &BTreeSet<EntityId>,
    actions: &mut Vec<EditorAction>,
) {
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
}
