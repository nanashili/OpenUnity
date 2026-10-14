//! Independent authoring controls and selectable forest rows.

mod row;

use std::collections::{BTreeMap, BTreeSet};

use eframe::egui::{ComboBox, ScrollArea, Ui};
use openunity_editor_core::scene::{EntityId, MAX_HIERARCHY_DEPTH, Primitive, Scene};
use openunity_editor_core::selection::Selection;

use crate::EditorAction;

/// Display the live forest and return document operations for the shell to validate.
pub fn show(ui: &mut Ui, scene: &Scene, selection: &Selection) -> Vec<EditorAction> {
    show_with_expansion(ui, scene, selection, &mut super::ExpansionState::default())
}

/// Render the authored hierarchy using the app-owned branch visibility state.
pub fn show_with_expansion(
    ui: &mut Ui,
    scene: &Scene,
    selection: &Selection,
    expansion: &mut super::ExpansionState,
) -> Vec<EditorAction> {
    let mut actions = Vec::new();
    let primary = selection.primary().filter(|id| scene.get(*id).is_some());
    let selected: BTreeSet<_> = selection.ids().collect();

    creation_controls(ui, scene, primary, selection, &selected, &mut actions);
    parent_control(ui, scene, primary, &mut actions);
    forest(ui, scene, &selected, expansion, &mut actions);
    actions
}

fn creation_controls(
    ui: &mut Ui,
    scene: &Scene,
    primary: Option<EntityId>,
    selection: &Selection,
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
        if ui
            .add_enabled(
                super::selection::can_group_selection(scene, selection),
                eframe::egui::Button::new("Create parent"),
            )
            .on_hover_text("Group the selected siblings under a new empty parent")
            .clicked()
        {
            actions.push(EditorAction::CreateParentForSelection);
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
    expansion: &mut super::ExpansionState,
    actions: &mut Vec<EditorAction>,
) {
    expansion.reconcile(scene);

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
                let sibling_order = scene.sibling_group(entity.parent).unwrap_or_default();
                let has_children = super::expansion::has_renderable_children(scene, id, depth);
                let collapsed = expansion.is_collapsed(id);
                let recursive_toggle = row::render_entity_row(
                    ui,
                    row::EntityRow {
                        entity,
                        depth,
                        sibling_position,
                        sibling_order,
                        has_children,
                        collapsed,
                    },
                    selected,
                    actions,
                );
                if let Some(recursive) = recursive_toggle {
                    expansion.toggle(scene, id, recursive);
                }
                if has_children
                    && !expansion.is_collapsed(id)
                    && depth + 1 < MAX_HIERARCHY_DEPTH
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
