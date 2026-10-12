//! Finite, value-owned 3D gravity configuration.

use engine_math::{Vector, finite};

/// Independent gravity configuration failures, not Unity native statuses.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GravityError {
    /// At least one requested gravity component is NaN or infinite.
    NonFinite,
}

/// One native context's gravity setting, independent of any simulation backend.
#[derive(Debug)]
pub struct GravityConfiguration {
    gravity: Vector,
}

impl Default for GravityConfiguration {
    fn default() -> Self {
        Self {
            gravity: [0.0, -9.81, 0.0],
        }
    }
}

impl GravityConfiguration {
    /// Return the complete finite gravity vector as an owned value.
    pub fn gravity(&self) -> Vector {
        self.gravity
    }

    /// Replace the complete gravity vector without arithmetic or clamping.
    /// # Errors
    /// Rejects nonfinite components without changing the previous setting.
    pub fn set_gravity(&mut self, gravity: Vector) -> Result<(), GravityError> {
        finite(&gravity).map_err(|_| GravityError::NonFinite)?;

        self.gravity = gravity;
        Ok(())
    }
}
