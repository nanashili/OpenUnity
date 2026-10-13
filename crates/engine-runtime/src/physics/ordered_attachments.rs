//! Deterministic attachment snapshots for whole-world operations.

use super::attachment_registry::{MAX_PHYSICS_ATTACHMENTS, PhysicsAttachment, PhysicsState};

impl PhysicsState {
    /// Copy attachments into deterministic backend identity order without allocating.
    pub(super) fn ordered_attachments(&self) -> impl Iterator<Item = PhysicsAttachment> {
        let mut entries: [Option<PhysicsAttachment>; MAX_PHYSICS_ATTACHMENTS] =
            self.attachments.entries;
        entries.sort_unstable_by(|left, right| match (left, right) {
            (Some(left), Some(right)) => left.body_id.to_raw().cmp(&right.body_id.to_raw()),
            (Some(_), None) => std::cmp::Ordering::Less,
            (None, Some(_)) => std::cmp::Ordering::Greater,
            (None, None) => std::cmp::Ordering::Equal,
        });

        entries.into_iter().flatten()
    }
}
