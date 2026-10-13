//! Stable slot reservation and commit/retirement for backend attachments.

use engine_identifiers::ObjectId;

use super::attachment_registry::{
    AttachmentIndex, AttachmentSlot, PhysicsAttachment, PhysicsState,
};
use super::owner_body_index::OwnerBodyIndexEntry;
use crate::RuntimeError;

impl PhysicsState {
    /// Check the owner limit and reserve a stable vacant slot before backend work.
    pub(super) fn prepare_attachment_slot(
        &self,
        owner_id: ObjectId,
    ) -> Result<AttachmentSlot, RuntimeError> {
        if self.attachments.owner_index.contains_owner(owner_id) {
            return Err(RuntimeError::PhysicsUnsupported);
        }

        let index = self
            .attachments
            .entries
            .iter()
            .enumerate()
            .position(|(index, attachment)| {
                attachment.is_none() && self.attachments.owner_index.is_empty_at(index)
            })
            .ok_or(RuntimeError::PhysicsUnsupported)?;

        Ok(AttachmentSlot { index, owner_id })
    }

    /// Reject an identity collision before reserving the matching backend body.
    pub(super) fn ensure_body_id_available(&self, body_id: ObjectId) -> Result<(), RuntimeError> {
        if self.attachment_by_body(body_id).is_some() {
            return Err(RuntimeError::InvalidObject);
        }
        Ok(())
    }

    /// Insert only after the ObjectStore and backend both hold the reservation.
    pub(super) fn insert_reserved_attachment(
        &mut self,
        slot: AttachmentSlot,
        attachment: PhysicsAttachment,
    ) {
        debug_assert_eq!(slot.owner_id, attachment.owner_id);
        debug_assert!(self.attachments.entries[slot.index].is_none());
        debug_assert!(
            !self
                .attachments
                .owner_index
                .contains_owner(attachment.owner_id)
        );
        debug_assert!(self.attachments.owner_index.is_empty_at(slot.index));
        debug_assert!(self.attachment_by_body(attachment.body_id).is_none());

        self.attachments.entries[slot.index] = Some(attachment);
        self.attachments.owner_index.insert(OwnerBodyIndexEntry {
            owner_id: attachment.owner_id,
            body_id: attachment.body_id,
            attachment_slot: slot.index,
        });
    }

    /// Complete a backend/object commit without a fallible lookup afterward.
    pub(super) fn commit_attachment(&mut self, index: AttachmentIndex, body_id: ObjectId) {
        let owner_entry = self
            .attachments
            .owner_index
            .entry_at(index.0)
            .expect("prepared attachment retains its owner index");
        let attachment = self.attachments.entries[index.0]
            .as_mut()
            .expect("prepared attachment remains in its registry slot");

        assert_eq!(attachment.body_id, body_id);
        assert_eq!(owner_entry.body_id, body_id);
        assert_eq!(owner_entry.owner_id, attachment.owner_id);
        assert_eq!(owner_entry.attachment_slot, index.0);

        attachment.committed = true;
    }

    /// Remove an exact attachment after all fallible backend work has succeeded.
    pub(super) fn remove_attachment(&mut self, index: AttachmentIndex, body_id: ObjectId) {
        let attachment = self.attachments.entries[index.0]
            .as_ref()
            .expect("prepared attachment remains in its registry slot");
        let owner_entry = self
            .attachments
            .owner_index
            .entry_at(index.0)
            .expect("prepared attachment retains its owner index");

        assert_eq!(attachment.body_id, body_id);
        assert_eq!(owner_entry.body_id, body_id);
        assert_eq!(owner_entry.owner_id, attachment.owner_id);
        assert_eq!(owner_entry.attachment_slot, index.0);

        self.attachments.entries[index.0] = None;
        self.attachments.owner_index.remove_at(index.0);
    }
}
