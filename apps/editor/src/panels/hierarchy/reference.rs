//! Compact authoring hierarchy over actual document identities.

use std::collections::BTreeMap;

use eframe::egui::{ComboBox, Ui};
use openunity_editor_core::scene::{Entity, EntityId, MAX_HIERARCHY_DEPTH, Primitive, Scene};
use openunity_editor_core::selection::Selection;

use super::super::hierarchy_browser::{HierarchyBrowser, Row};
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
    toolbar(ui, scene, primary, browser, &mut actions);
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
            .tree(ui, scene.name(), selection.ids())
            .into_iter()
            .map(|(id, toggle)| EditorAction::Select { id, toggle }),
    );
    actions
}

fn toolbar(
    ui: &mut Ui,
    scene: &Scene,
    primary: Option<EntityId>,
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
        });

        if let Some(descendants) = children.get(&Some(entity.id)) {
            let child_depth = depth
                .checked_add(1)
                .ok_or_else(|| "Native hierarchy depth overflow.".to_owned())?;
            if child_depth >= MAX_HIERARCHY_DEPTH {
                return Err("Native hierarchy exceeds its validated depth limit.".to_owned());
            }
            pending.extend(descendants.iter().rev().map(|e| (*e, child_depth)));
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
