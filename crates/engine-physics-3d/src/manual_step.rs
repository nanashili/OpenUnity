//! Private prepared motion using the approved independent kick-then-drift model.

use std::sync::Arc;

use engine_math::{Vector, add, finite, scale};
use engine_physics_api::{BodyId, BodyState, PhysicsError};

use crate::MotionWorld;
use crate::world::{BodyRecord, BodySlot, WorldProvenance};

/// An owned candidate bound to one world/revision, never a caller-forged commit.
#[derive(Debug)]
pub struct PreparedStep {
    provenance: Arc<WorldProvenance>,
    revision: u64,
    candidate: Option<BodyRecord>,
}

impl PreparedStep {
    /// Copy the candidate body for composition's Transform preflight/publication.
    /// Returns None for an empty world or an unpublished reservation.
    pub fn body(&self) -> Option<(BodyId, BodyState)> {
        self.candidate.map(|body| (body.id, body.state))
    }
}

impl MotionWorld {
    /// Prepare finite manual motion without changing any world field.
    ///
    /// Runtime owns Script-mode, active-owner and Transform admission. The
    /// backend validates duration/gravity even when no committed body exists.
    /// # Errors
    /// Returns Unsupported for nonzero angular velocity, Numeric for invalid
    /// inputs/result overflow, or revision exhaustion.
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

        let candidate = match self.slot {
            Some(BodySlot::Committed(body)) => {
                reject_unsupported_angular_motion(body.state)?;
                self.next_revision()?;
                Some(advance(body, duration, gravity)?)
            }
            Some(BodySlot::Reserved(_)) | None => None,
        };

        Ok(PreparedStep {
            provenance: Arc::clone(&self.provenance),
            revision: self.revision,
            candidate,
        })
    }

    /// Publish a matching prepared candidate without fallible work after mutation.
    /// # Errors
    /// Returns StalePlan for foreign/invalidated plans, or revision exhaustion.
    pub fn commit_step(&mut self, plan: PreparedStep) -> Result<(), PhysicsError> {
        if !Arc::ptr_eq(&self.provenance, &plan.provenance) || self.revision != plan.revision {
            return Err(PhysicsError::StalePlan);
        }
        if let Some(body) = plan.candidate {
            let revision = self.next_revision()?;

            self.slot = Some(BodySlot::Committed(body));
            self.revision = revision;
        }
        Ok(())
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
