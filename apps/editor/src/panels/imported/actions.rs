//! Supported requests emitted by imported document views.

use eframe::egui::Ui;
use openunity_editor_core::imported_scene::ImportedEntityEdit;
use openunity_editor_core::scene::EntityId;

/// Requests validated and committed by the app's imported-document dispatcher.
pub enum ImportedAction {
    /// Reference presentation request; the app must attach the displayed scope.
    SetComponentExpanded {
        /// Canonical supported component file ID.
        file_id: i64,
        /// Requested presentation state, independent of serialized data.
        expanded: bool,
    },
    /// Select a mapped handle, toggle it, or clear the current selection.
    Select {
        /// Document-local handle to select, or `None` to clear selection.
        id: Option<EntityId>,
        /// Whether to toggle the handle instead of replacing selection.
        toggle: bool,
    },
    /// Apply one complete, transactional GameObject/local Transform edit.
    EditEntity {
        /// Document-local owner of the GameObject and Transform properties.
        id: EntityId,
        /// Complete candidate properties validated together before commit.
        edit: ImportedEntityEdit,
    },
    /// Change an existing script's serialized enabled flag transactionally.
    SetScriptEnabled {
        /// Serialized file ID identifying the existing script component.
        script_file_id: i64,
        /// Candidate serialized enabled flag.
        enabled: bool,
    },
}

pub(super) fn selection_toggle(ui: &Ui) -> bool {
    ui.input(|input| input.modifiers.command || input.modifiers.ctrl)
}
