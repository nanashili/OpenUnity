//! Compact authoring hierarchy over actual document identities.

use std::collections::BTreeMap;

use eframe::egui::{ComboBox, Ui};
use openunity_editor_core::scene::{EntityId, MAX_HIERARCHY_DEPTH, Primitive, Scene};
use openunity_editor_core::selection::Selection;

use super::super::hierarchy_browser::{BrowserAction, HierarchyBrowser, Row};
use crate::EditorAction;

/// Compact reference presentation over the same actual forest and actions.
pub fn show_reference(
    ui: &mut Ui,
    scene: &Scene,
    selection: &Selection,
    browser: &mut HierarchyBrowser,
) -> Vec<EditorAction> {
    show_reference_inner(ui, scene, selection, browser, None)
}

pub(crate) fn show_reference_with_visibility(
    ui: &mut Ui,
    scene: &Scene,
    selection: &Selection,
    browser: &mut HierarchyBrowser,
    visibility: &mut crate::SceneViewState,
) -> Vec<EditorAction> {
    show_reference_inner(ui, scene, selection, browser, Some(visibility))
}

fn show_reference_inner(
    ui: &mut Ui,
    scene: &Scene,
    selection: &Selection,
    browser: &mut HierarchyBrowser,
    visibility: Option<&mut crate::SceneViewState>,
) -> Vec<EditorAction> {
    let mut actions = Vec::new();
    let primary = selection.primary().filter(|id| scene.get(*id).is_some());
    toolbar(ui, scene, primary, selection, browser, &mut actions);
    let revision = scene.hierarchy_revision();
    if let Err(error) = browser.refresh(revision, || forest_rows(scene)) {
        ui.colored_label(
            ui.visuals().error_fg_color,
            format!("Hierarchy unavailable: {error}"),
        );
        return actions;
    }

    actions.extend(
        browser
            .authored_tree(ui, scene, selection.ids(), visibility)
            .into_iter()
            .map(|action| match action {
                BrowserAction::Select(id, toggle) => EditorAction::Select { id, toggle },
                BrowserAction::Reparent(id, parent) => EditorAction::Reparent {
                    id,
                    parent: Some(parent),
                },
                BrowserAction::SetSiblingIndex(id, sibling_index) => {
                    EditorAction::SetSiblingIndex { id, sibling_index }
                }
            }),
    );
    actions
}

fn toolbar(
    ui: &mut Ui,
    scene: &Scene,
    primary: Option<EntityId>,
    selection: &Selection,
    browser: &mut HierarchyBrowser,
    actions: &mut Vec<EditorAction>,
) {
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
            if ui
                .add_enabled(
                    super::selection::can_group_selection(scene, selection),
                    eframe::egui::Button::new("Create empty parent"),
                )
                .clicked()
            {
                actions.push(EditorAction::CreateParentForSelection);
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
}

fn forest_rows(scene: &Scene) -> Result<Vec<Row>, String> {
    let expected_row_count = scene.entities().len();
    let entities: BTreeMap<_, _> = scene.entities().map(|entity| (entity.id, entity)).collect();
    let mut pending = Vec::new();
    if let Some(roots) = scene.sibling_group(None) {
        let root_count = roots.len();
        pending.extend(
            roots
                .iter()
                .enumerate()
                .rev()
                .map(|(index, id)| (*id, 0, Some((index, root_count)))),
        );
    }
    let mut rows = Vec::new();
    while let Some((id, depth, sibling_position)) = pending.pop() {
        let entity = entities
            .get(&id)
            .copied()
            .ok_or_else(|| format!("Hierarchy group references missing entity {}.", id.0))?;
        rows.push(Row {
            id: entity.id,
            parent: entity.parent,
            name: entity.name.clone(),
            depth,
            active: entity.active,
            sibling_position: (depth > 0).then_some(sibling_position).flatten(),
            root_position: (depth == 0).then_some(sibling_position).flatten(),
        });

        if let Some(descendants) = scene
            .sibling_group(Some(entity.id))
            .filter(|descendants| !descendants.is_empty())
        {
            let child_depth = depth
                .checked_add(1)
                .ok_or_else(|| "Native hierarchy depth overflow.".to_owned())?;
            if child_depth >= MAX_HIERARCHY_DEPTH {
                return Err("Native hierarchy exceeds its validated depth limit.".to_owned());
            }
            let sibling_count = descendants.len();
            pending.extend(
                descendants
                    .iter()
                    .enumerate()
                    .rev()
                    .map(|(index, id)| (*id, child_depth, Some((index, sibling_count)))),
            );
        }
    }

    if rows.len() != expected_row_count {
        return Err(format!(
            "Native hierarchy projected {} of {expected_row_count} entities.",
            rows.len()
        ));
    }

    Ok(rows)
}
