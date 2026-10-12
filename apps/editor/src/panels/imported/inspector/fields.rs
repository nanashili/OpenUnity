//! Exact local numeric widgets and read-only serialized/reference fields.

use super::super::ImportedAction;
use crate::appearance;
use eframe::egui::{self, Ui};
use openunity_editor_core::imported_scene::{FieldValue, ImportedDocument, ImportedReferenceKind};

pub(super) fn float_inputs<const N: usize>(
    ui: &mut Ui,
    name: &str,
    values: &mut [f32; N],
    controls: &mut Vec<serde_json::Value>,
) {
    if appearance::is_reference(ui) {
        ui.label(name);
        ui.horizontal(|ui| {
            let width = ((ui.available_width() - N as f32 * 16.0) / N as f32).max(25.0);
            for (axis, value) in ["X", "Y", "Z", "W"].into_iter().zip(values) {
                ui.push_id((name, axis), |ui| {
                    let label = ui.small(axis);
                    let response = ui
                        .add_sized([width, 18.0], egui::DragValue::new(value).speed(0.1))
                        .labelled_by(label.id)
                        .on_hover_text(format!("{name} {axis}"));
                    let property = match name {
                        "Local position" => "position",
                        "Raw local quaternion" => "rotation",
                        _ => "scale",
                    };
                    controls.push(crate::inspector_lock::control_json(
                        &format!(
                            "inspector.transform.{property}.{}",
                            axis.to_ascii_lowercase()
                        ),
                        &response,
                        false,
                    ));
                });
            }
        });
        return;
    }
    ui.label(name);
    ui.horizontal_wrapped(|ui| {
        for (axis, value) in ["X", "Y", "Z", "W"].into_iter().zip(values) {
            ui.push_id((name, axis), |ui| {
                let label = ui.label(axis);
                ui.add(egui::DragValue::new(value).speed(0.1))
                    .labelled_by(label.id)
                    .on_hover_text(format!("{name} {axis}"));
            });
        }
    });
}

pub(super) fn show_field(
    ui: &mut Ui,
    document: &ImportedDocument,
    name: &str,
    value: &FieldValue,
    actions: &mut Vec<ImportedAction>,
) {
    let (kind, text) = match value {
        FieldValue::I32(value) => ("i32", value.to_string()),
        FieldValue::F32(value) => ("f32", format!("{value:?}")),
        FieldValue::Bool(value) => ("bool", value.to_string()),
        FieldValue::String(value) => ("string", format!("{value:?}")),
        FieldValue::Vec3(value) => (
            "vec3",
            format!("X {:?} · Y {:?} · Z {:?}", value[0], value[1], value[2]),
        ),
        FieldValue::Quat(value) => (
            "quat",
            format!(
                "X {:?} · Y {:?} · Z {:?} · W {:?}",
                value[0], value[1], value[2], value[3]
            ),
        ),
        FieldValue::Reference(file_id) => {
            ui.label(format!("{name} (reference)"));
            match document.resolve_reference(*file_id) {
                Ok(None) => {
                    ui.label("fileID 0 · null");
                }
                Ok(Some(reference)) => {
                    let kind = match reference.kind {
                        ImportedReferenceKind::GameObject => "GameObject",
                        ImportedReferenceKind::Transform => "Transform",
                        ImportedReferenceKind::Script => "Script",
                    };
                    if ui
                        .button(format!(
                            "fileID {} · {kind} · {}",
                            reference.file_id, reference.name
                        ))
                        .on_hover_text(format!(
                            "Select canonical owner · editor ID {}",
                            reference.owner.0
                        ))
                        .clicked()
                    {
                        actions.push(ImportedAction::Select {
                            id: Some(reference.owner),
                            toggle: false,
                        });
                    }
                }
                Err(error) => {
                    ui.colored_label(ui.visuals().error_fg_color, error.to_string());
                }
            }
            return;
        }
    };
    if appearance::is_reference(ui) {
        // Keep wrapped values in their own column. horizontal_wrapped's text
        // continuation path can extend the parent left by the preceding label.
        ui.horizontal(|ui| {
            ui.add_sized([112.0, 16.0], egui::Label::new(name))
                .on_hover_text(format!("{name} ({kind}) · read-only"));
            ui.add(egui::Label::new(&text).wrap())
                .on_hover_text(format!("{kind}: {text}"));
        });
    } else {
        ui.label(format!("{name} ({kind})"));
        ui.label(text);
    }
}
