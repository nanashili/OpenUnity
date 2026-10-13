//! Exact-ID ownership, hidden reservations and atomic copied-state publication.

use std::sync::Arc;

use engine_math::{Quaternion, Vector};
use engine_physics_api::{BodyId, BodyState, PhysicsError};

use crate::body_registry::{BodyRecord, BodyRegistry, PreparedReplacements};

/// A private per-instance token: Arc identity binds plans without a global table.
#[derive(Debug)]
pub(crate) struct WorldProvenance;

/// A concrete collider-free world with bounded reserved and committed bodies.
///
/// The caller supplies fresh component identities and owns their external
/// registry/context validation. This world allocates no component identity.
/// Every mutation is atomic, and owned reads/plans contain no storage borrow.
#[derive(Debug)]
pub struct MotionWorld {
    pub(crate) provenance: Arc<WorldProvenance>,
    pub(crate) revision: u64,
    pub(crate) registry: BodyRegistry,
}

impl Default for MotionWorld {
    fn default() -> Self {
        Self {
            provenance: Arc::new(WorldProvenance),
            revision: 0,
            registry: BodyRegistry::default(),
        }
    }
}

impl MotionWorld {
    pub(crate) fn next_revision(&self) -> Result<u64, PhysicsError> {
        self.revision
            .checked_add(1)
            .ok_or(PhysicsError::RevisionExhausted)
    }

    /// Preflight one more reservation without advancing the world revision.
    /// # Errors
    /// Returns Capacity when the body limit is reached or storage cannot grow.
    pub fn ensure_reservation_capacity(&mut self) -> Result<(), PhysicsError> {
        self.registry.ensure_reservation_capacity()
    }

    /// Reserve a hidden body initialized from the caller's supported position.
    /// # Errors
    /// Returns Capacity before pose validation when no body slot is available.
    pub fn reserve(&mut self, id: BodyId, position: Vector) -> Result<(), PhysicsError> {
        self.reserve_pose(id, position, [0.0, 0.0, 0.0, 1.0])
    }

    /// Reserve a hidden body with an initial position and rotation.
    /// # Errors
    /// Returns Capacity, InvalidState for a duplicate ID, Numeric for invalid
    /// pose values, or RevisionExhausted before publication.
    pub fn reserve_pose(
        &mut self,
        id: BodyId,
        position: Vector,
        rotation: Quaternion,
    ) -> Result<(), PhysicsError> {
        self.registry.ensure_slot_available()?;
        let insertion_index = self.registry.insertion_index(id)?;
        self.registry.ensure_reservation_capacity()?;

        let mut state = BodyState::new(position)?;
        state.set_rotation(rotation)?;
        let revision = self.next_revision()?;

        self.registry
            .insert_reserved(insertion_index, BodyRecord { id, state });
        self.revision = revision;
        Ok(())
    }

    /// Publish a matching reservation exactly once, without callbacks/allocation.
    /// # Errors
    /// Returns InvalidBody/InvalidState for a mismatched slot, or exhaustion.
    pub fn commit(&mut self, id: BodyId) -> Result<(), PhysicsError> {
        let (index, body) = self.registry.reserved_body(id)?;
        let revision = self.next_revision()?;

        self.registry.publish_committed(index, body);
        self.revision = revision;
        Ok(())
    }

    /// Retire only a matching unpublished reservation.
    /// # Errors
    /// Returns InvalidBody/InvalidState for a mismatched slot, or exhaustion.
    pub fn abort(&mut self, id: BodyId) -> Result<(), PhysicsError> {
        let (index, _) = self.registry.reserved_body(id)?;
        let revision = self.next_revision()?;

        self.registry.remove_at(index);
        self.revision = revision;
        Ok(())
    }

    #[cfg(test)]
    pub(crate) fn reserved_body(&self, id: BodyId) -> Result<BodyRecord, PhysicsError> {
        self.registry.reserved_body(id).map(|(_, record)| record)
    }

    /// Validate that an exact body ID still names a hidden reservation.
    /// This read-only check does not advance the world revision.
    /// # Errors
    /// Returns InvalidBody for an absent ID or InvalidState for a committed ID.
    pub fn validate_reservation(&self, id: BodyId) -> Result<(), PhysicsError> {
        self.registry.reserved_body(id).map(|_| ())
    }

    /// Retire only a matching committed body, invalidating outstanding plans.
    /// # Errors
    /// Returns InvalidBody/InvalidState for a mismatched slot, or exhaustion.
    pub fn remove(&mut self, id: BodyId) -> Result<(), PhysicsError> {
        let (index, _) = self.registry.committed_body(id)?;
        let revision = self.next_revision()?;

        self.registry.remove_at(index);
        self.revision = revision;
        Ok(())
    }

    /// Atomically retire exact reserved or committed body IDs as one batch.
    /// # Errors
    /// Returns Capacity, InvalidBody/InvalidState, or RevisionExhausted.
    pub fn retire_bodies(&mut self, body_ids: &[BodyId]) -> Result<(), PhysicsError> {
        if body_ids.is_empty() {
            return Ok(());
        }

        let prepared = self.registry.prepare_retirement(body_ids)?;
        let revision = self.next_revision()?;

        self.registry.apply_retirement(prepared);
        self.revision = revision;
        Ok(())
    }

    /// Copy the complete finite state of a committed body.
    /// # Errors
    /// Returns InvalidBody for an absent ID, InvalidState for a reservation.
    pub fn body(&self, id: BodyId) -> Result<BodyState, PhysicsError> {
        Ok(self.registry.committed_body(id)?.1.state)
    }

    /// Copy every committed body in ascending positive ObjectId order.
    pub fn bodies(&self) -> impl Iterator<Item = (BodyId, BodyState)> + '_ {
        self.registry
            .committed_records()
            .map(|(_, record)| (record.id, record.state))
    }

    /// Publish a complete already validated state copy to one committed body.
    /// # Errors
    /// Returns InvalidBody/InvalidState for a mismatched slot, or exhaustion.
    pub fn set_body(&mut self, id: BodyId, state: BodyState) -> Result<(), PhysicsError> {
        let (index, _) = self.registry.committed_body(id)?;
        let revision = self.next_revision()?;

        self.registry
            .replace_committed(index, BodyRecord { id, state });
        self.revision = revision;
        Ok(())
    }

    /// Atomically publish copied states for exact committed IDs as one batch.
    /// An empty batch is a no-op; a nonempty batch advances revision once.
    /// # Errors
    /// Returns Capacity, InvalidBody/InvalidState, or RevisionExhausted.
    pub fn set_bodies(&mut self, bodies: &[(BodyId, BodyState)]) -> Result<(), PhysicsError> {
        if bodies.is_empty() {
            return Ok(());
        }
        if bodies.len() > crate::MAX_MOTION_BODIES {
            return Err(PhysicsError::Capacity);
        }

        let prepared: PreparedReplacements = self.registry.prepare_replacements(bodies)?;
        let revision = self.next_revision()?;

        self.registry.apply_replacements(prepared);
        self.revision = revision;
        Ok(())
    }
}

#[cfg(test)]
mod tests;
