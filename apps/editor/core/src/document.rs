//! Open-document history and selection scope for independent authoring scenes.

use crate::history::History;
use crate::scene::{Scene, SceneError};
use crate::selection::Selection;

/// One editor document session; replacing it begins a new identity scope.
pub struct Document {
    scene: Scene,
    saved: Option<Scene>,
    history: History<Scene>,
    selection: Selection,
}

impl Document {
    /// Start a new unsaved document.
    pub fn new(scene: Scene, history_capacity: usize) -> Self {
        Self {
            scene,
            saved: None,
            history: History::new(history_capacity),
            selection: Selection::new(),
        }
    }
    /// Read the validated live scene.
    pub fn scene(&self) -> &Scene {
        &self.scene
    }
    /// Current document-scoped selection.
    pub fn selection(&self) -> &Selection {
        &self.selection
    }
    /// Mutably access selection with its current scene for validation.
    pub fn selection_mut(&mut self) -> (&Scene, &mut Selection) {
        (&self.scene, &mut self.selection)
    }
    /// Whether the persisted document differs, including allocation high-water.
    pub fn is_dirty(&self) -> bool {
        self.saved.as_ref() != Some(&self.scene)
    }
    /// Mark a successfully persisted snapshot; callers never use this on save errors.
    pub fn mark_saved(&mut self) {
        self.saved = Some(self.scene.clone());
    }
    /// Replace a validated document and clear all old selection/history.
    pub fn replace(&mut self, scene: Scene) {
        self.scene = scene;
        self.saved = Some(self.scene.clone());
        self.history.clear();
        self.selection.clear();
    }
    /// Apply one user transaction, preserving all state on validation failure.
    pub fn edit(
        &mut self,
        label: &str,
        operation: impl FnOnce(&mut Scene) -> Result<(), SceneError>,
    ) -> Result<bool, SceneError> {
        let before = self.scene.clone();
        let mut after = before.clone();
        operation(&mut after)?;
        after.validate()?;
        if after == before {
            return Ok(false);
        }
        self.history.commit(label, before, after.clone());
        self.scene = after;
        self.selection.prune(&self.scene);
        Ok(true)
    }
    /// Whether an undo snapshot exists.
    pub fn can_undo(&self) -> bool {
        self.history.can_undo()
    }
    /// Whether a redo snapshot exists.
    pub fn can_redo(&self) -> bool {
        self.history.can_redo()
    }
    /// Restore history without lowering the session allocation counter.
    pub fn undo(&mut self) -> Result<bool, SceneError> {
        let Some(mut restored) = self.history.undo(&self.scene) else {
            return Ok(false);
        };
        restored.preserve_high_water(self.scene.next_id())?;
        self.scene = restored;
        self.selection.prune(&self.scene);
        Ok(true)
    }
    /// Restore redo without lowering the session allocation counter.
    pub fn redo(&mut self) -> Result<bool, SceneError> {
        let Some(mut restored) = self.history.redo(&self.scene) else {
            return Ok(false);
        };
        restored.preserve_high_water(self.scene.next_id())?;
        self.scene = restored;
        self.selection.prune(&self.scene);
        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scene::Primitive;
    #[test]
    fn branching_history_preserves_ids_and_document_scopes() {
        let mut document = Document::new(Scene::new("Untitled").unwrap(), 10);
        document
            .edit("Create", |s| {
                s.create("One", Primitive::Cube, None).map(|_| ())
            })
            .unwrap();
        let first = document.scene().entities().next().unwrap().id;
        {
            let (scene, selection) = document.selection_mut();
            selection.replace(Some(first), scene).unwrap();
        }
        document.undo().unwrap();
        assert_eq!(document.selection().primary(), None);
        document
            .edit("Create", |s| {
                s.create("Two", Primitive::Camera, None).map(|_| ())
            })
            .unwrap();
        let second = document.scene().entities().next().unwrap().id;
        assert!(second > first);
        assert!(!document.can_redo());
        document.mark_saved();
        assert!(!document.is_dirty());
        let replacement = Scene::new("Replacement").unwrap();
        document.replace(replacement);
        assert!(!document.can_undo());
        assert!(!document.is_dirty());
    }
    #[test]
    fn failed_and_noop_edits_preserve_history_and_saved_state() {
        let mut document = Document::new(Scene::new("Scene").unwrap(), 10);
        document.mark_saved();
        assert!(!document.edit("No-op", |_| Ok(())).unwrap());
        assert!(document.edit("Invalid", |s| s.set_name("")).is_err());
        assert!(!document.can_undo());
        assert!(!document.is_dirty());
    }
}
