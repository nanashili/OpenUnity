//! Whole-snapshot edits, history restoration, and saved-state tracking.

use crate::scene::EntityId;

use super::super::super::{ImportedEntityEdit, ImportedError, Result};
use super::super::ImportedSnapshot;
use super::ImportedDocument;

impl ImportedDocument {
    fn commit(&mut self, label: &str, mut candidate: ImportedSnapshot) -> Result<bool> {
        candidate.refresh()?;
        if candidate == self.snapshot {
            return Ok(false);
        }
        self.history
            .commit(label, self.snapshot.clone(), candidate.clone());
        self.snapshot = candidate;
        Ok(true)
    }
    /// Validate a cloned complete state and commit one supported entity edit.
    pub fn edit_entity(&mut self, id: EntityId, edit: ImportedEntityEdit) -> Result<bool> {
        let file = self.entity(id)?.game_object.file_id;
        let mut candidate = self.snapshot.clone();
        let g = candidate
            .plan
            .game_objects
            .iter_mut()
            .find(|g| g.file_id == file)
            .ok_or_else(|| ImportedError::new("edit imported entity", "missing GameObject"))?;
        g.name = edit.name;
        g.active = edit.active;
        let t = candidate
            .plan
            .transforms
            .iter_mut()
            .find(|t| t.owner == file)
            .ok_or_else(|| ImportedError::new("edit imported entity", "missing Transform"))?;
        t.local_position = edit.local_position;
        t.local_rotation = edit.local_rotation;
        t.local_scale = edit.local_scale;
        self.commit("Edit imported entity", candidate)
    }
    /// Validate and commit an existing script enabled-state edit.
    pub fn set_script_enabled(&mut self, file_id: i64, enabled: bool) -> Result<bool> {
        let mut candidate = self.snapshot.clone();
        let script = candidate
            .plan
            .scripts
            .iter_mut()
            .find(|s| s.file_id == file_id)
            .ok_or_else(|| {
                ImportedError::new("edit imported script", "missing script file identity")
            })?;
        script.enabled = enabled;
        self.commit("Set imported script enabled", candidate)
    }
    /// Whether a committed transaction can be undone.
    pub fn can_undo(&self) -> bool {
        self.history.can_undo()
    }
    /// Whether an undone transaction can be restored.
    pub fn can_redo(&self) -> bool {
        self.history.can_redo()
    }
    /// Label of the next undo transaction.
    pub fn undo_label(&self) -> Option<&str> {
        self.history.undo_label()
    }
    /// Label of the next redo transaction.
    pub fn redo_label(&self) -> Option<&str> {
        self.history.redo_label()
    }
    /// Restore the preceding whole-state snapshot without changing selection.
    pub fn undo(&mut self) -> Result<bool> {
        if let Some(mut snapshot) = self.history.undo(&self.snapshot) {
            snapshot.next_id = snapshot.next_id.max(self.snapshot.next_id);
            self.snapshot = snapshot;
            Ok(true)
        } else {
            Ok(false)
        }
    }
    /// Restore the following whole-state snapshot without changing selection.
    pub fn redo(&mut self) -> Result<bool> {
        if let Some(mut snapshot) = self.history.redo(&self.snapshot) {
            snapshot.next_id = snapshot.next_id.max(self.snapshot.next_id);
            self.snapshot = snapshot;
            Ok(true)
        } else {
            Ok(false)
        }
    }
    /// Whether the exact state differs from the last successful save.
    pub fn is_dirty(&self) -> bool {
        self.saved.as_ref() != Some(&self.snapshot)
    }
    /// Record the current exact state after a successful independent save.
    pub fn mark_saved(&mut self) {
        self.saved = Some(self.snapshot.clone());
    }
}
