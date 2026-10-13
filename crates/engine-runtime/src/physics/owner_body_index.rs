//! Fixed-capacity exact owner-to-body associations for Runtime attachments.

use engine_identifiers::ObjectId;

use super::attachment_registry::MAX_PHYSICS_ATTACHMENTS;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct OwnerBodyIndexEntry {
    pub(super) owner_id: ObjectId,
    pub(super) body_id: ObjectId,
    pub(super) attachment_slot: usize,
}

#[derive(Debug)]
pub(super) struct OwnerBodyIndex {
    entries: [Option<OwnerBodyIndexEntry>; MAX_PHYSICS_ATTACHMENTS],
}

impl OwnerBodyIndex {
    pub(super) fn contains_owner(&self, owner_id: ObjectId) -> bool {
        self.entries
            .iter()
            .flatten()
            .any(|entry| entry.owner_id == owner_id)
    }

    pub(super) fn entry_for_owner(&self, owner_id: ObjectId) -> Option<OwnerBodyIndexEntry> {
        self.entries
            .iter()
            .flatten()
            .find(|entry| entry.owner_id == owner_id)
            .copied()
    }

    pub(super) fn entry_at(&self, attachment_slot: usize) -> Option<OwnerBodyIndexEntry> {
        self.entries[attachment_slot]
    }

    pub(super) fn is_empty_at(&self, attachment_slot: usize) -> bool {
        self.entries[attachment_slot].is_none()
    }

    pub(super) fn insert(&mut self, entry: OwnerBodyIndexEntry) {
        self.entries[entry.attachment_slot] = Some(entry);
    }

    pub(super) fn remove_at(&mut self, attachment_slot: usize) {
        self.entries[attachment_slot] = None;
    }
}

impl Default for OwnerBodyIndex {
    fn default() -> Self {
        Self {
            entries: [None; MAX_PHYSICS_ATTACHMENTS],
        }
    }
}
