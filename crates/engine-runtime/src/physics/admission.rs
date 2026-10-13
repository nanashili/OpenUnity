//! Native motion admission for the runtime fixed-mode boundary.

use engine_physics_api::SimulationMode;

use crate::{Runtime, RuntimeError};

impl Runtime {
    /// Reject fixed stepping once a body has been committed.
    pub(crate) fn physics_fixed_admission(&self) -> Result<(), RuntimeError> {
        if self.physics.mode == SimulationMode::Fixed && self.physics.has_committed_attachments() {
            return Err(RuntimeError::PhysicsUnsupported);
        }

        Ok(())
    }
}
