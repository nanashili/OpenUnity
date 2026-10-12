//! Exhaustive Transform and script component rendering.

use super::super::ImportedAction;
use super::{
    Draft,
    fields::{float_inputs, show_field},
};
use crate::appearance;
use eframe::egui::{self, Ui};
use openunity_editor_core::imported_scene::{ImportedComponent, ImportedDocument};

pub(super) fn show(
    ui: &mut Ui,
    document: &ImportedDocument,
    draft: &mut Draft,
    controls: &mut Vec<serde_json::Value>,
    actions: &mut Vec<ImportedAction>,
    component: ImportedComponent<'_>,
    scoped: Option<(crate::inspector_lock::InspectorTarget, bool)>,
) -> egui::Response {
    let compact = appearance::is_reference(ui);
    let file_id = match component {
        ImportedComponent::Transform(t) => t.file_id,
        ImportedComponent::Script(s) => s.file_id,
    };
    match component {
        ImportedComponent::Transform(transform) => {
            component_header(
                format!("Transform · fileID {}", transform.file_id),
                "imported_transform",
                file_id,
                scoped,
            )
            .show(ui, |ui| {
                if compact {
                    egui::CollapsingHeader::new("Source Transform details")
                        .id_salt(("transform_source", transform.file_id))
                        .icon(crate::resources::disclosure)
                        .show(ui, |ui| {
                            ui.label(format!(
                                "Owner fileID {} · parent Transform fileID {}",
                                transform.owner, transform.parent
                            ));
                            ui.label(format!(
                                "Ordered child Transform fileIDs: {:?}",
                                transform.children
                            ));
                            ui.monospace(format!(
                                "Stored local position: {:?}",
                                transform.local_position
                            ));
                            ui.monospace(format!(
                                "Stored raw quaternion: {:?}",
                                transform.local_rotation
                            ));
                            ui.monospace(format!(
                                "Stored local scale: {:?}",
                                transform.local_scale
                            ));
                        });
                } else {
                    ui.label(format!(
                        "Owner fileID {} · parent Transform fileID {}",
                        transform.owner, transform.parent
                    ));
                    ui.label(format!(
                        "Ordered child Transform fileIDs: {:?}",
                        transform.children
                    ));
                    ui.monospace(format!(
                        "Stored local position: {:?}",
                        transform.local_position
                    ));
                    ui.monospace(format!(
                        "Stored raw quaternion: {:?}",
                        transform.local_rotation
                    ));
                    ui.monospace(format!(
                        "Stored local scale: {:?}",
                        transform.local_scale
                    ));
                }

                float_inputs(ui, "Local position", &mut draft.edit.local_position, controls);
                float_inputs(
                    ui,
                    "Raw local quaternion",
                    &mut draft.edit.local_rotation,
                    controls,
                );
                float_inputs(ui, "Local scale", &mut draft.edit.local_scale, controls);
                if compact {
                    ui.small("Raw quaternion · exact components").on_hover_text(
                        "Quaternion components are stored exactly; no Euler conversion or normalization.",
                    );
                } else {
                    ui.small("Quaternion components are stored exactly; no Euler conversion or normalization.");
                }
            })
            .header_response
        }
        ImportedComponent::Script(script) => {
            let full_header =
                format!("Script {} · fileID {}", script.type_name, script.file_id);
            let header = if compact {
                format!(
                    "{} · {}",
                    script
                        .type_name
                        .rsplit(['.', '+'])
                        .next()
                        .unwrap_or(&script.type_name),
                    script.file_id
                )
            } else {
                full_header.clone()
            };
            component_header(header, "imported_script", file_id, scoped)
                .show(ui, |ui| {
                    if compact {
                        egui::CollapsingHeader::new("Source script details")
                            .icon(crate::resources::disclosure)
                            .id_salt(("script_source", script.file_id))
                            .show(ui, |ui| {
                                ui.label(format!("Owner fileID {}", script.owner));
                                ui.label(format!("GUID: {}", script.guid));
                                ui.label(format!("Assembly: {}", script.assembly));
                                ui.add(
                                    egui::Label::new(format!("Type: {}", script.type_name))
                                        .wrap(),
                                );
                                ui.label(format!(
                                    "Execution order: {}",
                                    script.execution_order
                                ));
                            });
                    } else {
                        ui.label(format!("Owner fileID {}", script.owner));
                        ui.label(format!("GUID: {}", script.guid));
                        ui.label(format!("Assembly: {}", script.assembly));
                        ui.label(format!("Type: {}", script.type_name));
                        ui.label(format!("Execution order: {}", script.execution_order));
                    }
                    let mut enabled = script.enabled;
                    if ui.checkbox(&mut enabled, "Enabled").changed() {
                        actions.push(ImportedAction::SetScriptEnabled {
                            script_file_id: script.file_id,
                            enabled,
                        });
                    }
                    egui::CollapsingHeader::new("Serialized fields (read-only)")
                        .icon(crate::resources::disclosure)
                        .id_salt(("imported_fields", script.file_id))
                        .default_open(true)
                        .show(ui, |ui| {
                            for field in &script.fields {
                                ui.push_id(&field.name, |ui| {
                                    show_field(
                                        ui,
                                        document,
                                        &field.name,
                                        &field.value,
                                        actions,
                                    );
                                });
                            }
                            if script.fields.is_empty() {
                                ui.weak("No serialized fields.");
                            }
                        });
                })
                .header_response
                .on_hover_text(full_header)
        }
    }
}

fn component_header(
    label: String,
    kind: &str,
    file_id: i64,
    scoped: Option<(crate::inspector_lock::InspectorTarget, bool)>,
) -> egui::CollapsingHeader {
    let header = egui::CollapsingHeader::new(label)
        .icon(crate::resources::disclosure)
        .default_open(true);
    if let Some((target, expanded)) = scoped {
        let salt = (
            "inspector.component",
            target.document_scope,
            target.entity_id.0,
            file_id,
        );
        // Let the header resolve its own child-UI identity. Individual clicks
        // emit scoped requests above; the shared controller supplies this state.
        header.id_salt(salt).open(Some(expanded))
    } else {
        header.id_salt((kind, file_id))
    }
}
