//! Authored-order hierarchy presentation and canonical handle selection.

use eframe::egui::{self, Ui};
use openunity_editor_core::imported_scene::ImportedDocument;

use super::super::hierarchy_browser::{HierarchyBrowser, Row};
use super::{ImportedAction, actions::selection_toggle};

/// Compact tree; structural controls remain unsupported for imported documents.
pub fn show_hierarchy_reference(
    ui: &mut Ui,
    document: &ImportedDocument,
    browser: &mut HierarchyBrowser,
) -> Vec<ImportedAction> {
    ui.horizontal(|ui| {
        ui.add_enabled(false, egui::Button::new("+"))
            .on_disabled_hover_text(
                "Create, duplicate, delete and reparent are unsupported for imported documents.",
            );
        browser.search(ui);
    });
    let mut pending: Vec<_> = document
        .roots()
        .into_iter()
        .rev()
        .map(|id| (id, None, 0))
        .collect();
    let mut rows = Vec::new();
    while let Some((id, parent, depth)) = pending.pop() {
        match document.entity(id) {
            Ok(entity) => rows.push(Row {
                id,
                parent,
                name: entity.game_object.name.clone(),
                depth,
                active: entity.game_object.active,
                selected: document.selection().contains(&id),
            }),
            Err(error) => {
                ui.colored_label(ui.visuals().error_fg_color, error.to_string());
                continue;
            }
        }
        match document.children(id) {
            Ok(children) => pending.extend(
                children
                    .into_iter()
                    .rev()
                    .map(|child| (child, Some(id), depth + 1)),
            ),
            Err(error) => {
                ui.colored_label(ui.visuals().error_fg_color, error.to_string());
            }
        }
    }
    browser
        .tree(ui, &document.plan().scene_name, &rows)
        .into_iter()
        .map(|(id, toggle)| ImportedAction::Select {
            id: Some(id),
            toggle,
        })
        .collect()
}

/// Show roots and children in their authored order, using document-local handles.
pub fn show_hierarchy(ui: &mut Ui, document: &ImportedDocument) -> Vec<ImportedAction> {
    let mut actions = Vec::new();
    ui.small("Imported hierarchy · authored order");
    ui.horizontal_wrapped(|ui| {
        for label in ["Create", "Duplicate", "Delete", "Reparent / reorder"] {
            ui.add_enabled(false, egui::Button::new(label))
                .on_disabled_hover_text("Structural edits are unsupported for imported documents.");
        }
    });
    let mut pending: Vec<_> = document
        .roots()
        .into_iter()
        .rev()
        .map(|id| (id, 0))
        .collect();
    egui::ScrollArea::both()
        .id_salt("imported_hierarchy")
        .show(ui, |ui| {
            while let Some((id, depth)) = pending.pop() {
                let entity = match document.entity(id) {
                    Ok(entity) => entity,
                    Err(error) => {
                        ui.colored_label(ui.visuals().error_fg_color, error.to_string());
                        continue;
                    }
                };
                ui.push_id(id.0, |ui| {
                    ui.horizontal(|ui| {
                        ui.add_space(depth as f32 * 14.0);
                        let response = ui
                            .selectable_label(
                                document.selection().contains(&id),
                                &entity.game_object.name,
                            )
                            .on_hover_text(format!(
                                "GameObject fileID {} · editor ID {}",
                                entity.game_object.file_id, id.0
                            ));
                        if response.clicked() {
                            actions.push(ImportedAction::Select {
                                id: Some(id),
                                toggle: selection_toggle(ui),
                            });
                        }
                        if !entity.game_object.active {
                            ui.weak("inactive");
                        }
                    });
                });
                match document.children(id) {
                    Ok(children) => {
                        pending.extend(children.into_iter().rev().map(|id| (id, depth + 1)))
                    }
                    Err(error) => {
                        ui.colored_label(ui.visuals().error_fg_color, error.to_string());
                    }
                }
            }
        });
    actions
}
