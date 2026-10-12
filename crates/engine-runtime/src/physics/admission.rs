//! Native motion admission for the runtime fixed-mode boundary.

use engine_physics_api::SimulationMode;

use crate::{Runtime, RuntimeError};

impl Runtime {
    /// Reject fixed stepping once a body has been committed.
    pub(crate) fn physics_fixed_admission(&self) -> Result<(), RuntimeError> {
        let fixed_body = self
            .physics
            .attachment
            .is_some_and(|attachment| attachment.committed);

        if self.physics.mode == SimulationMode::Fixed && fixed_body {
            return Err(RuntimeError::PhysicsUnsupported);
        }

        Ok(())
    }
}
