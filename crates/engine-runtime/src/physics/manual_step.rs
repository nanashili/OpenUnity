//! Runtime composition for Script-mode manual physics simulation.
//!
//! It delegates numerical motion and transaction invalidation to the existing
//! `engine-physics-3d::MotionWorld` prepared-step API.

use engine_identifiers::ObjectId;
use engine_physics_api::{BodyId, SimulationMode};

use super::body_properties::validate_profile;
use crate::{Runtime, RuntimeError};

/// Runtime facts captured by the attachment owner during complete preflight.
#[derive(Clone, Copy, Debug)]
pub(super) struct ManualStepAdmission {
    pub(super) body_id: ObjectId,
    pub(super) owner_id: ObjectId,
    pub(super) transform_id: ObjectId,
}

impl Runtime {
    /// Run one Script-mode step and publish its copied position atomically.
    ///
    /// `manual_step_admission` checks mode, duration, owner activity and
    /// pending state, and reads profile/dirty revision facts through the
    /// attachment's `BodyProperties` methods before checking body identity.
    /// The Transform owner then validates and borrows the exact target before
    /// backend commit, leaving only an infallible position assignment after it.
    pub fn physics_simulate(&mut self, duration: f32) -> Result<(), RuntimeError> {
        let admission = self.manual_step_admission(duration)?;
        let gravity = self.physics_gravity();
        let plan = self
            .physics
            .world
            .prepare_step(duration, gravity)
            .map_err(RuntimeError::from)?;

        let Some(admission) = admission else {
            self.physics
                .world
                .commit_step(plan)
                .map_err(RuntimeError::from)?;
            return Ok(());
        };

        let Some((candidate_id, candidate_state)) = plan.body() else {
            return Err(RuntimeError::InvalidObject);
        };
        if candidate_id != admission.body_id.into() {
            return Err(RuntimeError::InvalidObject);
        }

        let (world, transforms) = (&mut self.physics.world, &mut self.transforms);
        let target = Runtime::checked_physics_transform_target(
            transforms,
            admission.transform_id,
            admission.owner_id,
        )?;

        world.commit_step(plan).map_err(RuntimeError::from)?;

        target.position = candidate_state.position();
        Ok(())
    }

    /// Preflight all runtime state before preparing a backend step.
    ///
    /// Lifecycle admission owns ancestry and pending-request checks. The
    /// lifetime owner owns the supported root Transform check; the property
    /// owner supplies the copied body state and profile validation.
    pub(super) fn manual_step_admission(
        &self,
        duration: f32,
    ) -> Result<Option<ManualStepAdmission>, RuntimeError> {
        self.lifecycle_ready()?;
        if self.scene_staging() {
            return Err(RuntimeError::StagingBusy);
        }
        if self.physics.mode != SimulationMode::Script {
            return Err(RuntimeError::PhysicsUnsupported);
        }
        validate_duration(duration)?;

        let Some(attachment) = self.physics.attachment else {
            return Ok(None);
        };
        if !attachment.committed {
            return Ok(None);
        }

        self.validate_physics_step_owner(attachment.owner_id, attachment.body_id)?;
        self.validate_supported_owner_transform(attachment.owner_id, attachment.transform_id)?;
        if attachment.is_dirty() {
            return Err(RuntimeError::Lifecycle);
        }

        let state = self.rigidbody_state(attachment.body_id)?;
        validate_profile(state.configuration()).map_err(RuntimeError::from)?;
        self.physics
            .world
            .body(BodyId::from(attachment.body_id))
            .map_err(RuntimeError::from)?;

        Ok(Some(ManualStepAdmission {
            body_id: attachment.body_id,
            owner_id: attachment.owner_id,
            transform_id: attachment.transform_id,
        }))
    }
}

fn validate_duration(duration: f32) -> Result<(), RuntimeError> {
    if !duration.is_finite() || duration <= 0.0 {
        return Err(RuntimeError::Numeric);
    }
    Ok(())
}
