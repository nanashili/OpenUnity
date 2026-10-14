//! Independent authoring controls and selectable forest rows.

use std::collections::{BTreeMap, BTreeSet};

use eframe::egui::{ComboBox, ScrollArea, Ui};
use openunity_editor_core::scene::{EntityId, MAX_HIERARCHY_DEPTH, Primitive, Scene};
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
    let entities: BTreeMap<_, _> = scene.entities().map(|entity| (entity.id, entity)).collect();
    ScrollArea::both()
        .id_salt("hierarchy_forest")
        .show(ui, |ui| {
            if scene.entities().len() == 0 {
                ui.label("Scene is empty. Add an entity to begin.");
            }
            let mut pending = Vec::new();
            let roots = scene.sibling_group(None).unwrap_or(&[]);
            if !roots.is_empty() {
                let root_count = roots.len();
                pending.extend(
                    roots
                        .iter()
                        .enumerate()
                        .rev()
                        .map(|(index, id)| (*id, 0, Some((index, root_count)))),
                );
            }
            while let Some((id, depth, sibling_position)) = pending.pop() {
                let Some(entity) = entities.get(&id).copied() else {
                    continue;
                };
                render_entity_row(
                    ui,
                    entity,
                    depth,
                    sibling_position,
                    roots,
                    selected,
                    actions,
                );
                if depth + 1 < MAX_HIERARCHY_DEPTH
                    && let Some(descendants) = scene.sibling_group(Some(entity.id))
                {
                    let sibling_count = descendants.len();
                    pending.extend(
                        descendants.iter().enumerate().rev().map(|(index, child)| {
                            (*child, depth + 1, Some((index, sibling_count)))
                        }),
                    );
                }
            }
        });
}

fn render_entity_row(
    ui: &mut Ui,
    entity: &openunity_editor_core::scene::Entity,
    depth: usize,
    sibling_position: Option<(usize, usize)>,
    root_order: &[EntityId],
    selected: &BTreeSet<EntityId>,
    actions: &mut Vec<EditorAction>,
) {
    let row_response = ui.push_id(entity.id.0, |ui| {
        ui.horizontal(|ui| {
            ui.add_space(depth as f32 * 14.0);
            let label = format!(
                "{} ({})",
                entity.name,
                if entity.active { "active" } else { "inactive" }
            );
            let response = if depth == 0 {
                ui.add(
                    eframe::egui::Button::selectable(selected.contains(&entity.id), label)
                        .sense(eframe::egui::Sense::click_and_drag())
                        .min_size(eframe::egui::vec2(ui.available_width(), 18.0))
                        .frame(false),
                )
            } else {
                ui.selectable_label(selected.contains(&entity.id), label)
            };
            if depth > 0 {
                sibling_controls(ui, entity.id, sibling_position, actions);
            }
            response
        })
    });
    let response = row_response.inner.inner;

    if depth == 0 {
        let drop = super::super::root_drag::observe_root_row(
            &response,
            entity.id,
            sibling_position,
            root_order,
            true,
        );
        if let Some((dragged_id, sibling_index)) = drop.operation {
            actions.push(EditorAction::SetSiblingIndex {
                id: dragged_id,
                sibling_index,
            });
            return;
        }
        if drop.completed {
            return;
        }
    }

    if response.clicked() {
        let toggle = ui.input(|input| input.modifiers.command || input.modifiers.ctrl);
        actions.push(EditorAction::Select {
            id: entity.id,
            toggle,
        });
    }
}

fn sibling_controls(
    ui: &mut Ui,
    id: EntityId,
    sibling_position: Option<(usize, usize)>,
    actions: &mut Vec<EditorAction>,
) {
    let Some((sibling_index, sibling_count)) = sibling_position else {
        return;
    };
    if sibling_count < 2 {
        return;
    }

    if sibling_index > 0
        && ui
            .small_button("↟")
            .on_hover_text("Set as first sibling")
            .clicked()
    {
        actions.push(EditorAction::SetSiblingIndex {
            id,
            sibling_index: 0,
        });
    }
    if sibling_index + 1 < sibling_count
        && ui
            .small_button("↡")
            .on_hover_text("Set as last sibling")
            .clicked()
    {
        actions.push(EditorAction::SetSiblingIndex {
            id,
            sibling_index: sibling_count - 1,
        });
    }
}
