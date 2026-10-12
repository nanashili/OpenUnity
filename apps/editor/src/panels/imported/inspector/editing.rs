//! Whole-entity fields, validation summary and guarded Apply actions.

use eframe::egui::{self, Ui};
use openunity_editor_core::imported_scene::ImportedEntityView;
use openunity_editor_core::scene::EntityId;

use super::super::ImportedAction;
use super::{Draft, draft_error, same_edit};

pub(super) fn identity(
    ui: &mut Ui,
    entity: &ImportedEntityView<'_>,
    id: EntityId,
    selected_count: usize,
    compact: bool,
) {
    if !compact {
        ui.heading("Imported Inspector");
    }
    if selected_count > 1 {
        ui.label(format!(
            "{} selected; editing the primary GameObject.",
            selected_count
        ));
    }
    let identity = format!(
        "GameObject fileID {} · editor ID {}",
        entity.game_object.file_id, id.0
    );
    if compact {
        ui.add(egui::Label::new(egui::RichText::new(&identity).small()).truncate())
            .on_hover_text(&identity);
    } else {
        ui.label(identity);
    }
}

pub(super) fn properties(
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
                    egui::TextEdit::singleline(&mut draft.edit.name),
                )
                .labelled_by(label.id);
            controls.push(crate::inspector_lock::control_json(
                "inspector.name",
                &response,
                false,
            ));
        } else {
            ui.text_edit_singleline(&mut draft.edit.name)
                .labelled_by(label.id);
        }
    });
    ui.checkbox(&mut draft.edit.active, "Active self");
    if compact {
        ui.separator();
    } else {
        ui.small(
            "Components follow the authored component order. Fields and references are read-only.",
        );
    }
}

pub(super) fn footer(
    ui: &mut Ui,
    draft: &Draft,
    id: EntityId,
    controls: &mut Vec<serde_json::Value>,
    actions: &mut Vec<ImportedAction>,
    compact: bool,
) {
    let error = draft_error(&draft.edit);
    if let Some(error) = error {
        let response = ui.colored_label(ui.visuals().error_fg_color, error);
        if compact {
            controls.push(crate::inspector_lock::control_json(
                "inspector.validation_error",
                &response,
                false,
            ));
        }
    }
    let apply = egui::Button::new("Apply imported properties");
    let apply = ui.add_enabled(
        !same_edit(&draft.edit, &draft.original) && error.is_none(),
        apply,
    );
    if compact {
        controls.push(crate::inspector_lock::control_json(
            "inspector.apply",
            &apply,
            false,
        ));
    }
    if apply.clicked() {
        actions.push(ImportedAction::EditEntity {
            id,
            edit: draft.edit.clone(),
        });
    }
    ui.add_enabled(
        false,
        egui::Button::new("Edit serialized fields / references"),
    )
    .on_disabled_hover_text(
        "Serialized field/reference edits are unsupported; values remain fully inspectable.",
    );
    ui.add_enabled(false, egui::Button::new("Euler rotation / primitive / component edits"))
            .on_disabled_hover_text("Imported documents preserve raw quaternions and authored components; these edits are unsupported.");
}
