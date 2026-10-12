//! Target resolution and ordered imported Inspector rendering.

use crate::appearance;
use eframe::egui::Ui;
use openunity_editor_core::imported_scene::{
    ImportedDocument, ImportedEntityEdit, ImportedEntityView,
};
use openunity_editor_core::scene::EntityId;

use super::super::ImportedAction;
use super::{Draft, InspectorPanel, same_edit};
use super::{editing, sections};

impl InspectorPanel {
    pub(super) fn show_impl(
        &mut self,
        ui: &mut Ui,
        document: &ImportedDocument,
        explicit: Option<Option<EntityId>>,
        expansion: Option<&crate::component_expansion::ComponentExpansionSnapshot>,
    ) -> Vec<ImportedAction> {
        self.controls.clear();
        let mut actions = Vec::new();
        let compact = appearance::is_reference(ui);

        let Some((id, entity)) = self.prepare_target(ui, document, explicit, compact) else {
            return actions;
        };
        let Some(draft) = self.draft.as_mut() else {
            return actions;
        };

        editing::identity(ui, &entity, id, self.selected.len(), compact);
        editing::properties(ui, draft, &mut self.controls, compact);

        let components = match document.components(id) {
            Ok(components) => components,
            Err(error) => {
                ui.colored_label(ui.visuals().error_fg_color, error.to_string());
                return actions;
            }
        };

        sections::show(
            ui,
            document,
            draft,
            &mut self.controls,
            &mut actions,
            expansion,
            components,
        );

        editing::footer(ui, draft, id, &mut self.controls, &mut actions, compact);
        actions
    }

    fn prepare_target<'a>(
        &mut self,
        ui: &mut Ui,
        document: &'a ImportedDocument,
        explicit: Option<Option<EntityId>>,
        compact: bool,
    ) -> Option<(EntityId, ImportedEntityView<'a>)> {
        let selected: Vec<_> = explicit.map_or_else(
            || document.selection().iter().copied().collect(),
            |id| id.into_iter().collect(),
        );
        let Some(id) = explicit.unwrap_or_else(|| document.primary()) else {
            self.selected = selected;
            self.draft = None;
            if !compact {
                ui.heading("Imported Inspector");
            }
            ui.label("Select an imported GameObject to inspect its complete components.");
            return None;
        };
        let entity = match document.entity(id) {
            Ok(entity) => entity,
            Err(error) => {
                self.draft = None;
                ui.colored_label(ui.visuals().error_fg_color, error.to_string());
                return None;
            }
        };
        self.reconcile_draft(id, selected, &entity, explicit);
        Some((id, entity))
    }

    fn reconcile_draft(
        &mut self,
        id: EntityId,
        selected: Vec<EntityId>,
        entity: &ImportedEntityView<'_>,
        explicit: Option<Option<EntityId>>,
    ) {
        let source = ImportedEntityEdit {
            name: entity.game_object.name.clone(),
            active: entity.game_object.active,
            local_position: entity.transform.local_position,
            local_rotation: entity.transform.local_rotation,
            local_scale: entity.transform.local_scale,
        };
        if self.selected != selected
            || self.draft.as_ref().is_none_or(|draft| {
                draft.id != id
                    || (!same_edit(&draft.original, &source)
                        && (explicit.is_none() || same_edit(&draft.edit, &draft.original)))
            })
        {
            self.selected = selected;
            self.draft = Some(Draft {
                id,
                original: source.clone(),
                edit: source,
            });
        }
    }
}
