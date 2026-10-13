//! Bounded Runtime ownership of Rigidbody-to-owner/Transform relationships.
//!
//! Body state remains in `MotionWorld`. This registry stores only identity,
//! publication state, and authored-position revision metadata. Reserved
//! attachments occupy capacity just like committed attachments. Its explicit
//! owner-to-body index enforces one body per owner across both states.

use engine_identifiers::ObjectId;
use engine_physics_3d::{AabbQueryWorld, MotionWorld};
use engine_physics_api::SimulationMode;

use crate::RuntimeError;

use super::body_properties::BodyProperties;
use super::owner_body_index::{OwnerBodyIndex, OwnerBodyIndexEntry};

pub(super) const MAX_PHYSICS_ATTACHMENTS: usize = 256;

/// Private relationship between a Rigidbody identity and its owner Transform.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct PhysicsAttachment {
    pub(super) body_id: ObjectId,
    pub(super) owner_id: ObjectId,
    pub(super) transform_id: ObjectId,
    pub(super) committed: bool,
    pub(super) properties: BodyProperties,
}

impl PhysicsAttachment {
    pub(super) fn reserved(
        body_id: ObjectId,
        owner_id: ObjectId,
        transform_id: ObjectId,
        properties: BodyProperties,
    ) -> Self {
        Self {
            body_id,
            owner_id,
            transform_id,
            committed: false,
            properties,
        }
    }

    pub(super) fn is_dirty(self) -> bool {
        self.properties.is_position_dirty()
    }
}

/// Runtime-owned backend state and bounded attachment registry.
#[derive(Debug, Default)]
pub(crate) struct PhysicsState {
    pub(super) world: MotionWorld,
    pub(super) query: AabbQueryWorld,
    pub(super) mode: SimulationMode,
    pub(super) attachments: AttachmentRegistry,
}

impl PhysicsState {
    pub(super) fn attachment_by_body(&self, body_id: ObjectId) -> Option<&PhysicsAttachment> {
        self.attachments
            .entries
            .iter()
            .flatten()
            .find(|attachment| attachment.body_id == body_id)
    }

    pub(super) fn attachment_by_body_mut(
        &mut self,
        body_id: ObjectId,
    ) -> Option<&mut PhysicsAttachment> {
        self.attachments
            .entries
            .iter_mut()
            .flatten()
            .find(|attachment| attachment.body_id == body_id)
    }

    pub(super) fn attachment_by_owner(&self, owner_id: ObjectId) -> Option<&PhysicsAttachment> {
        let owner_entry = self.attachments.owner_index.entry_for_owner(owner_id)?;

        let attachment = self.attachments.entries[owner_entry.attachment_slot]
            .as_ref()
            .expect("owner index points to a live attachment slot");
        assert_eq!(attachment.owner_id, owner_entry.owner_id);
        assert_eq!(attachment.body_id, owner_entry.body_id);

        Some(attachment)
    }

    pub(super) fn attachment_by_transform(
        &self,
        transform_id: ObjectId,
    ) -> Option<&PhysicsAttachment> {
        self.attachments
            .entries
            .iter()
            .flatten()
            .find(|attachment| attachment.transform_id == transform_id)
    }

    pub(super) fn attachment_by_transform_mut(
        &mut self,
        transform_id: ObjectId,
    ) -> Option<&mut PhysicsAttachment> {
        self.attachments
            .entries
            .iter_mut()
            .flatten()
            .find(|attachment| attachment.transform_id == transform_id)
    }

    pub(super) fn attachments(&self) -> impl Iterator<Item = &PhysicsAttachment> + '_ {
        self.attachments.entries.iter().flatten()
    }

    pub(super) fn has_committed_attachments(&self) -> bool {
        self.attachments
            .entries
            .iter()
            .flatten()
            .any(|attachment| attachment.committed)
    }

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

    pub(super) fn attachment_index_by_body(&self, body_id: ObjectId) -> Option<AttachmentIndex> {
        self.attachments
            .entries
            .iter()
            .position(|entry| entry.is_some_and(|attachment| attachment.body_id == body_id))
            .map(AttachmentIndex)
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

#[derive(Clone, Copy, Debug)]
pub(super) struct AttachmentSlot {
    index: usize,
    owner_id: ObjectId,
}

#[derive(Clone, Copy, Debug)]
pub(super) struct AttachmentIndex(usize);

#[derive(Debug)]
pub(super) struct AttachmentRegistry {
    pub(super) entries: [Option<PhysicsAttachment>; MAX_PHYSICS_ATTACHMENTS],
    owner_index: OwnerBodyIndex,
}

impl Default for AttachmentRegistry {
    fn default() -> Self {
        Self {
            entries: [None; MAX_PHYSICS_ATTACHMENTS],
            owner_index: OwnerBodyIndex::default(),
        }
    }
}
