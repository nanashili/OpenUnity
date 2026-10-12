//! Safe access to the context-owned 3D gravity configuration.

use crate::{Runtime, RuntimeError};
use engine_math::Vector;
use engine_physics_api::GravityError;

impl Runtime {
    /// Return the complete configured gravity value without advancing simulation.
    pub fn physics_gravity(&self) -> Vector {
        self.gravity.gravity()
    }

    /// Replace configured gravity after validating every component.
    /// # Errors
    /// Nonfinite input returns `Numeric` and preserves the complete previous value.
    pub fn set_physics_gravity(&mut self, gravity: Vector) -> Result<(), RuntimeError> {
        self.gravity
            .set_gravity(gravity)
            .map_err(|error| match error {
                GravityError::NonFinite => RuntimeError::Numeric,
            })
    }
}
