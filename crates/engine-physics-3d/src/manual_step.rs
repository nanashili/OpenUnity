//! Private prepared motion using the approved independent kick-then-drift model.

use std::sync::Arc;

use engine_math::{Vector, add, finite, scale};
use engine_physics_api::{BodyId, BodyState, PhysicsError};

use crate::body_registry::BodyRecord;
use crate::world::WorldProvenance;
use crate::{MAX_MOTION_BODIES, MotionWorld};

#[derive(Clone, Copy, Debug)]
struct StepCandidate {
    slot_index: usize,
    body: BodyRecord,
}

/// An owned candidate set bound to one world/revision, never caller-forged.
#[derive(Debug)]
pub struct PreparedStep {
    provenance: Arc<WorldProvenance>,
    revision: u64,
    candidates: Vec<StepCandidate>,
}

impl PreparedStep {
    /// Copy the sole candidate for legacy one-body callers.
    /// Returns None for zero or multiple committed candidates.
    pub fn body(&self) -> Option<(BodyId, BodyState)> {
        if self.candidates.len() != 1 {
            return None;
        }

        let body = self.candidates[0].body;
        Some((body.id, body.state))
    }

    /// Copy every candidate in ascending positive ObjectId order.
    pub fn bodies(&self) -> impl Iterator<Item = (BodyId, BodyState)> + '_ {
        self.candidates
            .iter()
            .map(|candidate| (candidate.body.id, candidate.body.state))
    }
}

impl MotionWorld {
    /// Prepare finite manual motion without changing any world field.
    ///
    /// Runtime owns Script-mode, active-owner and Transform admission. The
    /// backend validates duration/gravity even when no committed body exists.
    /// # Errors
    /// Returns Unsupported for nonzero angular velocity, Numeric for invalid
    /// inputs/result overflow, Capacity for candidate allocation, or exhaustion.
    pub fn prepare_step(
        &self,
        duration: f32,
        gravity: Vector,
    ) -> Result<PreparedStep, PhysicsError> {
        finite(&[duration])?;
        if duration <= 0.0 {
            return Err(PhysicsError::Numeric);
        }
        finite(&gravity)?;

        for (_, body) in self.registry.committed_records() {
            reject_unsupported_angular_motion(body.state)?;
        }

        let candidate_count = self.registry.committed_count();
        if candidate_count == 0 {
            return Ok(self.prepared_step(Vec::new()));
        }

        self.next_revision()?;

        let mut candidates = Vec::new();
        candidates
            .try_reserve_exact(candidate_count.min(MAX_MOTION_BODIES))
            .map_err(|_| PhysicsError::Capacity)?;

        for (slot_index, body) in self.registry.committed_records() {
            let advanced = advance(body, duration, gravity)?;
            candidates.push(StepCandidate {
                slot_index,
                body: advanced,
            });
        }

        Ok(self.prepared_step(candidates))
    }

    /// Publish a matching prepared candidate set without fallible work after mutation.
    /// # Errors
    /// Returns StalePlan for foreign/invalidated plans, or revision exhaustion.
    pub fn commit_step(&mut self, plan: PreparedStep) -> Result<(), PhysicsError> {
        if !Arc::ptr_eq(&self.provenance, &plan.provenance) || self.revision != plan.revision {
            return Err(PhysicsError::StalePlan);
        }
        if plan.candidates.is_empty() {
            return Ok(());
        }

        let revision = self.next_revision()?;
        for candidate in plan.candidates {
            self.registry
                .replace_committed(candidate.slot_index, candidate.body);
        }
        self.revision = revision;
        Ok(())
    }

    fn prepared_step(&self, candidates: Vec<StepCandidate>) -> PreparedStep {
        PreparedStep {
            provenance: Arc::clone(&self.provenance),
            revision: self.revision,
            candidates,
        }
    }
}

fn reject_unsupported_angular_motion(state: BodyState) -> Result<(), PhysicsError> {
    let angular_velocity = state.configuration().angular_velocity();
    if angular_velocity.iter().any(|component| *component != 0.0) {
        return Err(PhysicsError::Unsupported);
    }
    Ok(())
}

fn advance(
    mut body: BodyRecord,
    duration: f32,
    gravity: Vector,
) -> Result<BodyRecord, PhysicsError> {
    let old_velocity = body.state.linear_velocity();
    let candidate_velocity = if body.state.configuration().use_gravity() {
        let delta_velocity = scale(gravity, [duration; 3])?;
        add(old_velocity, delta_velocity)?
    } else {
        old_velocity
    };
    let delta_position = scale(candidate_velocity, [duration; 3])?;
    let candidate_position = add(body.state.position(), delta_position)?;

    body.state.set_linear_velocity(candidate_velocity)?;
    body.state.set_position(candidate_position)?;
    Ok(body)
}
