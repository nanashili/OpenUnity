//! Live authored component membership and shared expansion actions.

use crate::appearance;
use eframe::egui::Ui;
use openunity_editor_core::imported_scene::{ImportedComponent, ImportedDocument};

use super::super::ImportedAction;
use super::{Draft, components};

pub(super) fn show(
    ui: &mut Ui,
    document: &ImportedDocument,
    draft: &mut Draft,
    controls: &mut Vec<serde_json::Value>,
    actions: &mut Vec<ImportedAction>,
    expansion: Option<&crate::component_expansion::ComponentExpansionSnapshot>,
    components: Vec<ImportedComponent<'_>>,
) {
    let id = draft.id;
    let compact = appearance::is_reference(ui);
    for component in components {
        let file_id = match component {
            ImportedComponent::Transform(t) => t.file_id,
            ImportedComponent::Script(s) => s.file_id,
        };
        let scoped = expansion.and_then(|state| {
            state
                .target
                .filter(|target| target.entity_id == id)
                .and_then(|target| {
                    state
                        .sections
                        .iter()
                        .find(|s| s.file_id == file_id)
                        .map(|section| (target, section.expanded))
                })
        });
        if compact && expansion.is_some() && scoped.is_none() {
            continue;
        }
        let response = components::show(ui, document, draft, controls, actions, component, scoped);
        if let Some((_, expanded)) = scoped {
            controls.push(crate::inspector_lock::control_json(
                &format!("inspector.component.{file_id}"),
                &response,
                expanded,
            ));
            if response.clicked() {
                actions.push(ImportedAction::SetComponentExpanded {
                    file_id,
                    expanded: !expanded,
                });
            }
        }
    }
}
