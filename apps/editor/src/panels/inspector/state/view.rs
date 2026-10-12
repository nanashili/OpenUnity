//! Authoring Inspector layout, fields, validation and Apply action.

use eframe::egui::{self, Ui};
use openunity_editor_core::scene::{EntityId, Scene};
use openunity_editor_core::selection::Selection;

use super::{Draft, InspectorPanel};
use crate::{EditorAction, appearance};

impl InspectorPanel {
    pub(super) fn show_impl(
        &mut self,
        ui: &mut Ui,
        scene: &Scene,
        selection: &Selection,
        explicit: Option<Option<EntityId>>,
    ) -> Vec<EditorAction> {
        self.controls.clear();
        self.reconcile_draft(scene, selection, explicit);

        let compact = appearance::is_reference(ui);
        if !compact {
            ui.heading("Inspector");
        }
        let Some(draft) = &mut self.draft else {
            ui.label("Select an entity to inspect its local properties.");
            return Vec::new();
        };

        identity(ui, draft, self.selected.len(), compact);
        properties(ui, draft, &mut self.controls, compact);

        let error = draft.validation_error();
        if let Some(error) = error {
            ui.colored_label(ui.visuals().error_fg_color, error);
        }
        let apply = ui.add_enabled(
            draft.changed() && error.is_none(),
            egui::Button::new("Apply"),
        );
        if compact {
            self.controls.push(crate::inspector_lock::control_json(
                "inspector.apply",
                &apply,
                false,
            ));
        }
        if apply.clicked() {
            return vec![EditorAction::Edit {
                id: draft.source.id,
                name: draft.name.clone(),
                active: draft.active,
                transform: draft.transform,
            }];
        }
        Vec::new()
    }
}

fn identity(ui: &mut Ui, draft: &Draft, selected_count: usize, compact: bool) {
    if selected_count > 1 {
        ui.label(format!(
            "{} entities selected. Editing only the primary entity.",
            selected_count
        ));
    }
    if compact {
        ui.small(format!(
            "{:?} · ID {}",
            draft.source.primitive, draft.source.id.0
        ))
        .on_hover_text(match draft.source.parent {
            Some(parent) => format!("Parent: {}", parent.0),
            None => "Parent: none (root)".into(),
        });
    } else {
        ui.label(format!("ID: {}", draft.source.id.0));
        ui.label(format!("Primitive: {:?}", draft.source.primitive));
        ui.label(match draft.source.parent {
            Some(parent) => format!("Parent: {}", parent.0),
            None => "Parent: none (root)".to_owned(),
        });
        ui.separator();
    }
}

fn properties(
    ui: &mut Ui,
    draft: &mut Draft,
    controls: &mut Vec<serde_json::Value>,
    compact: bool,
) {
    ui.horizontal(|ui| {
        let label = ui.label("Name");
        if compact {
            let response = ui
                .add_sized(
                    [ui.available_width(), 18.0],
                    egui::TextEdit::singleline(&mut draft.name),
                )
                .labelled_by(label.id);
            controls.push(crate::inspector_lock::control_json(
                "inspector.name",
                &response,
                false,
            ));
        } else {
            ui.text_edit_singleline(&mut draft.name)
                .labelled_by(label.id);
        }
    });
    ui.checkbox(&mut draft.active, "Active");
    if compact {
        ui.separator();
        ui.strong("Transform");
    }
    transform_row(ui, "Translation", &mut draft.transform.translation);
    transform_row(ui, "Rotation (degrees)", &mut draft.transform.rotation_deg);
    transform_row(ui, "Scale", &mut draft.transform.scale);
}

fn transform_row(ui: &mut Ui, name: &str, values: &mut [f32; 3]) {
    if appearance::is_reference(ui) {
        ui.horizontal(|ui| {
            ui.add_sized([94.0, 18.0], egui::Label::new(name));
            let width = ((ui.available_width() - 38.0) / 3.0).max(26.0);
            for (axis, value) in ["X", "Y", "Z"].into_iter().zip(values) {
                ui.push_id((name, axis), |ui| {
                    let label = ui.small(axis);
                    ui.add_sized([width, 18.0], egui::DragValue::new(value).speed(0.1))
                        .labelled_by(label.id)
                        .on_hover_text(format!("{name} {axis}"));
                });
            }
        });
        return;
    }
    ui.label(name);
    ui.horizontal_wrapped(|ui| {
        for (axis, value) in ["X", "Y", "Z"].into_iter().zip(values) {
            ui.push_id((name, axis), |ui| {
                let label = ui.label(axis);
                ui.add(egui::DragValue::new(value).speed(0.1))
                    .labelled_by(label.id)
                    .on_hover_text(format!("{name} {axis}"));
            });
        }
    });
}
