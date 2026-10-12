//! Private imported draft state, exact numeric identity and validation.

use eframe::egui::Ui;
use openunity_editor_core::imported_scene::{ImportedDocument, ImportedEntityEdit};
use openunity_editor_core::scene::EntityId;

use super::ImportedAction;

mod components;
mod editing;
mod fields;
mod sections;
mod view;

/// Uncommitted imported-property drafts, reset on replacement by the app.
#[derive(Default)]
pub struct InspectorPanel {
    selected: Vec<EntityId>,
    draft: Option<Draft>,
    controls: Vec<serde_json::Value>,
}

struct Draft {
    id: EntityId,
    original: ImportedEntityEdit,
    edit: ImportedEntityEdit,
}

fn same_edit(a: &ImportedEntityEdit, b: &ImportedEntityEdit) -> bool {
    a.name == b.name
        && a.active == b.active
        && a.local_position
            .iter()
            .chain(&a.local_rotation)
            .chain(&a.local_scale)
            .zip(
                b.local_position
                    .iter()
                    .chain(&b.local_rotation)
                    .chain(&b.local_scale),
            )
            .all(|(a, b)| a.to_bits() == b.to_bits())
}

fn draft_error(edit: &ImportedEntityEdit) -> Option<&'static str> {
    if edit.name.len() > 1024 * 1024 || edit.name.contains('\0') {
        return Some("Name must contain no NUL and be at most 1 MiB of UTF-8.");
    }
    if edit
        .local_position
        .iter()
        .chain(&edit.local_rotation)
        .chain(&edit.local_scale)
        .any(|value| !value.is_finite())
    {
        return Some("All local Transform components must be finite.");
    }
    if edit.local_rotation.iter().all(|value| *value == 0.0) {
        return Some("The local Transform quaternion must be nonzero.");
    }
    None
}

impl InspectorPanel {
    /// Inspect complete ordered components and return only supported edit requests.
    pub fn show(&mut self, ui: &mut Ui, document: &ImportedDocument) -> Vec<ImportedAction> {
        self.show_impl(ui, document, None, None)
    }

    /// Inspect an explicitly scoped app target without following global selection.
    pub(crate) fn show_target(
        &mut self,
        ui: &mut Ui,
        document: &ImportedDocument,
        target: Option<EntityId>,
        expansion: &crate::component_expansion::ComponentExpansionSnapshot,
    ) -> Vec<ImportedAction> {
        self.show_impl(ui, document, Some(target), Some(expansion))
    }

    pub(crate) fn draft_state(&self) -> (Option<EntityId>, bool) {
        (
            self.draft.as_ref().map(|d| d.id),
            self.draft
                .as_ref()
                .is_some_and(|d| !same_edit(&d.edit, &d.original)),
        )
    }

    pub(crate) fn controls(&self) -> &[serde_json::Value] {
        &self.controls
    }

    pub(crate) fn draft_snapshot(&self) -> serde_json::Value {
        self.draft.as_ref().map_or(serde_json::Value::Null, |d| {
            serde_json::json!({
                "entity_id": d.id.0,
                "name": d.edit.name,
                "original_name": d.original.name,
                "changed": !same_edit(&d.edit, &d.original),
                "active": d.edit.active,
                "local_position": d.edit.local_position,
                "local_rotation": d.edit.local_rotation,
                "local_scale": d.edit.local_scale,
                "original_position": d.original.local_position,
                "original_rotation": d.original.local_rotation,
                "original_scale": d.original.local_scale,
                "validation_error": draft_error(&d.edit)
            })
        })
    }
}
