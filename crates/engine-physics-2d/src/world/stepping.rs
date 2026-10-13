//! Transactional caller-timed simulation steps.

use engine_physics_api::CollisionError;

use super::{CollisionWorld2D, MAX_TIMESTEP_SECONDS};
use crate::body_snapshot;
use crate::validation::validate_position;

impl CollisionWorld2D {
    /// Advance the Rapier solver and commit only finite body state.
    ///
    /// # Errors
    /// Returns InvalidTimestep for a nonfinite, nonpositive, or above-maximum
    /// timestep, or InvalidNumber if the solver produces nonfinite body state
    /// or a position outside the supported range.
    pub fn step(&mut self, timestep: f32) -> Result<(), CollisionError> {
        if !timestep.is_finite() || timestep <= 0.0 || timestep > MAX_TIMESTEP_SECONDS {
            return Err(CollisionError::InvalidTimestep);
        }

        let mut candidate = self.clone();
        candidate.step_candidate(timestep)?;

        *self = candidate;
        Ok(())
    }

    fn step_candidate(&mut self, timestep: f32) -> Result<(), CollisionError> {
        self.integration_parameters.dt = timestep;
        self.pipeline.step(
            &self.gravity,
            &self.integration_parameters,
            &mut self.islands,
            &mut self.broad_phase,
            &mut self.narrow_phase,
            &mut self.rigid_bodies,
            &mut self.colliders,
            &mut self.impulse_joints,
            &mut self.multibody_joints,
            &mut self.ccd_solver,
            None,
            &(),
            &(),
        );

        self.validate_body_states()
    }

    fn validate_body_states(&self) -> Result<(), CollisionError> {
        for handle in self.body_handles.values() {
            let body = self
                .rigid_bodies
                .get(*handle)
                .ok_or(CollisionError::InvalidBody)?;
            let snapshot = body_snapshot::copy_from(body)?;
            validate_position(snapshot.position())?;
        }

        Ok(())
    }
}
