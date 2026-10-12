//! Private authoring draft ownership and reconciliation.

use eframe::egui::Ui;
use openunity_editor_core::scene::{Entity, EntityId, MAX_NAME_BYTES, Scene, Transform};
use openunity_editor_core::selection::Selection;

use crate::EditorAction;

#[cfg(test)]
mod tests;
mod view;

/// Inspector drafts stay local until the shell executes an Apply action.
#[derive(Default)]
pub struct InspectorPanel {
    selected: Vec<EntityId>,
    draft: Option<Draft>,
    controls: Vec<serde_json::Value>,
}

struct Draft {
    source: Entity,
    name: String,
    active: bool,
    transform: Transform,
}

impl Draft {
    fn new(source: &Entity) -> Self {
        Self {
            source: source.clone(),
            name: source.name.clone(),
            active: source.active,
            transform: source.transform,
        }
    }

    fn changed(&self) -> bool {
        self.name != self.source.name
            || self.active != self.source.active
            || self.transform != self.source.transform
    }

    fn validation_error(&self) -> Option<&'static str> {
        if self.name.is_empty() {
            return Some("Edit entity: name must not be empty.");
        }
        if self.name.len() > MAX_NAME_BYTES {
            return Some("Edit entity: name exceeds 1024 UTF-8 bytes.");
        }
        if self
            .transform
            .translation
            .iter()
            .chain(&self.transform.rotation_deg)
            .chain(&self.transform.scale)
            .any(|value| !value.is_finite())
        {
            return Some("Edit entity: all transform values must be finite.");
        }
        None
    }
}

impl InspectorPanel {
    /// Render the primary selection and return at most one complete edit.
    ///
    /// Selection changes and external entity edits replace any uncommitted draft.
    pub fn show(&mut self, ui: &mut Ui, scene: &Scene, selection: &Selection) -> Vec<EditorAction> {
        self.show_impl(ui, scene, selection, None)
    }

    /// Render one explicitly scoped app target without following global selection.
    pub(crate) fn show_target(
        &mut self,
        ui: &mut Ui,
        scene: &Scene,
        selection: &Selection,
        target: Option<EntityId>,
    ) -> Vec<EditorAction> {
        self.show_impl(ui, scene, selection, Some(target))
    }

    pub(crate) fn draft_state(&self) -> (Option<EntityId>, bool) {
        (
            self.draft.as_ref().map(|d| d.source.id),
            self.draft.as_ref().is_some_and(Draft::changed),
        )
    }
    pub(crate) fn controls(&self) -> &[serde_json::Value] {
        &self.controls
    }
    pub(crate) fn draft_snapshot(&self) -> serde_json::Value {
        self.draft.as_ref().map_or(serde_json::Value::Null, |d| {
            serde_json::json!({
                "entity_id": d.source.id.0,
                "name": d.name,
                "original_name": d.source.name,
                "changed": d.changed()
            })
        })
    }

    fn reconcile_draft(
        &mut self,
        scene: &Scene,
        selection: &Selection,
        explicit: Option<Option<EntityId>>,
    ) {
        let selected: Vec<_> =
            explicit.map_or_else(|| selection.ids().collect(), |id| id.into_iter().collect());
        let entity = explicit
            .unwrap_or_else(|| selection.primary())
            .and_then(|id| scene.get(id));
        let source_changed = self.draft.as_ref().map(|draft| &draft.source) != entity;
        let dirty = self.draft.as_ref().is_some_and(Draft::changed);
        if self.selected != selected || (source_changed && (explicit.is_none() || !dirty)) {
            self.selected = selected;
            self.draft = entity.map(Draft::new);
        }
    }
}
