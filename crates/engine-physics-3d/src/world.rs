//! One-body ownership, hidden reservations and atomic copied-state publication.

use std::sync::Arc;

use engine_math::Vector;
use engine_physics_api::{BodyId, BodyState, PhysicsError};

/// A private per-instance token: Arc identity binds plans without a global table.
#[derive(Debug)]
pub(crate) struct WorldProvenance;

#[derive(Clone, Copy, Debug)]
pub(crate) struct BodyRecord {
    pub(crate) id: BodyId,
    pub(crate) state: BodyState,
}

#[derive(Clone, Copy, Debug)]
pub(crate) enum BodySlot {
    Reserved(BodyRecord),
    Committed(BodyRecord),
}

/// A concrete collider-free world with capacity for one reserved/committed body.
///
/// The caller supplies fresh component identities and owns their external
/// registry/context validation. This world allocates no component identity.
/// Every mutation is atomic, and owned reads/plans contain no storage borrow.
#[derive(Debug)]
pub struct MotionWorld {
    pub(crate) provenance: Arc<WorldProvenance>,
    pub(crate) revision: u64,
    pub(crate) slot: Option<BodySlot>,
}

impl Default for MotionWorld {
    fn default() -> Self {
        Self {
            provenance: Arc::new(WorldProvenance),
            revision: 0,
            slot: None,
        }
    }
}

impl MotionWorld {
    pub(crate) fn next_revision(&self) -> Result<u64, PhysicsError> {
        self.revision
            .checked_add(1)
            .ok_or(PhysicsError::RevisionExhausted)
    }

    /// Reserve a hidden body initialized from the caller's supported position.
    /// # Errors
    /// Returns Capacity if occupied, Numeric for nonfinite position, or exhaustion.
    pub fn reserve(&mut self, id: BodyId, position: Vector) -> Result<(), PhysicsError> {
        if self.slot.is_some() {
            return Err(PhysicsError::Capacity);
        }
        let state = BodyState::new(position)?;
        let revision = self.next_revision()?;

        self.slot = Some(BodySlot::Reserved(BodyRecord { id, state }));
        self.revision = revision;
        Ok(())
    }

    /// Publish a matching reservation exactly once, without callbacks/allocation.
    /// # Errors
    /// Returns InvalidBody/InvalidState for a mismatched slot, or exhaustion.
    pub fn commit(&mut self, id: BodyId) -> Result<(), PhysicsError> {
        let body = self.reserved_body(id)?;
        let revision = self.next_revision()?;

        self.slot = Some(BodySlot::Committed(body));
        self.revision = revision;
        Ok(())
    }

    /// Retire only a matching unpublished reservation.
    /// # Errors
    /// Returns InvalidBody/InvalidState for a mismatched slot, or exhaustion.
    pub fn abort(&mut self, id: BodyId) -> Result<(), PhysicsError> {
        self.reserved_body(id)?;
        self.clear_slot()
    }

    /// Retire only a matching committed body, invalidating outstanding plans.
    /// # Errors
    /// Returns InvalidBody/InvalidState for a mismatched slot, or exhaustion.
    pub fn remove(&mut self, id: BodyId) -> Result<(), PhysicsError> {
        self.committed_body(id)?;
        self.clear_slot()
    }

    fn clear_slot(&mut self) -> Result<(), PhysicsError> {
        let revision = self.next_revision()?;

        self.slot = None;
        self.revision = revision;
        Ok(())
    }

    /// Copy the complete finite state of a committed body.
    /// # Errors
    /// Returns InvalidBody for an absent ID, InvalidState for a reservation.
    pub fn body(&self, id: BodyId) -> Result<BodyState, PhysicsError> {
        Ok(self.committed_body(id)?.state)
    }

    /// Publish a complete already validated state copy to a committed body.
    /// # Errors
    /// Returns InvalidBody/InvalidState for a mismatched slot, or exhaustion.
    pub fn set_body(&mut self, id: BodyId, state: BodyState) -> Result<(), PhysicsError> {
        self.committed_body(id)?;
        let revision = self.next_revision()?;

        self.slot = Some(BodySlot::Committed(BodyRecord { id, state }));
        self.revision = revision;
        Ok(())
    }

    fn reserved_body(&self, id: BodyId) -> Result<BodyRecord, PhysicsError> {
        match self.slot {
            Some(BodySlot::Reserved(body)) if body.id == id => Ok(body),
            Some(BodySlot::Committed(body)) if body.id == id => Err(PhysicsError::InvalidState),
            _ => Err(PhysicsError::InvalidBody),
        }
    }

    pub(crate) fn committed_body(&self, id: BodyId) -> Result<BodyRecord, PhysicsError> {
        match self.slot {
            Some(BodySlot::Committed(body)) if body.id == id => Ok(body),
            Some(BodySlot::Reserved(body)) if body.id == id => Err(PhysicsError::InvalidState),
            _ => Err(PhysicsError::InvalidBody),
        }
    }
}
