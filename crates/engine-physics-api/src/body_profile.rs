//! Atomic supported-profile replacements of owned body configuration.

use crate::{BodyConfiguration, BodyInterpolation, PhysicsError};
use engine_math::{Vector, finite};

fn supported_zero(value: f32) -> Result<(), PhysicsError> {
    finite(&[value])?;
    if value != 0.0 {
        return Err(PhysicsError::Unsupported);
    }
    Ok(())
}

impl BodyConfiguration {
    /// Replace mass. Reject nonfinite or nonpositive values without mutation.
    /// # Errors
    /// Returns Numeric for an invalid mass.
    pub fn set_mass(&mut self, mass: f32) -> Result<(), PhysicsError> {
        finite(&[mass])?;
        if mass <= 0.0 {
            return Err(PhysicsError::Numeric);
        }
        self.mass = mass;
        Ok(())
    }

    /// Replace the gravity participation flag on this owned value.
    pub fn set_use_gravity(&mut self, enabled: bool) {
        self.use_gravity = enabled;
    }

    /// Store zero angular velocity without enabling rotation integration.
    /// # Errors
    /// Returns Numeric for nonfinite inputs, Unsupported for finite nonzero inputs.
    pub fn set_angular_velocity(&mut self, velocity: Vector) -> Result<(), PhysicsError> {
        finite(&velocity)?;
        if velocity.iter().any(|value| *value != 0.0) {
            return Err(PhysicsError::Unsupported);
        }
        self.angular_velocity = velocity;
        Ok(())
    }

    /// Store the supported dynamic-body flag.
    /// # Errors
    /// Returns Unsupported for a kinematic body request.
    pub fn set_is_kinematic(&mut self, enabled: bool) -> Result<(), PhysicsError> {
        if enabled {
            return Err(PhysicsError::Unsupported);
        }
        self.is_kinematic = enabled;
        Ok(())
    }

    /// Store the supported collider-free flag.
    /// # Errors
    /// Returns Unsupported for collision detection requests.
    pub fn set_detect_collisions(&mut self, enabled: bool) -> Result<(), PhysicsError> {
        if enabled {
            return Err(PhysicsError::Unsupported);
        }
        self.detect_collisions = enabled;
        Ok(())
    }

    /// Store zero linear damping.
    /// # Errors
    /// Returns Numeric for nonfinite inputs, Unsupported for finite nonzero inputs.
    pub fn set_linear_damping(&mut self, damping: f32) -> Result<(), PhysicsError> {
        supported_zero(damping)?;
        self.linear_damping = damping;
        Ok(())
    }

    /// Store zero angular damping.
    /// # Errors
    /// Returns Numeric for nonfinite inputs, Unsupported for finite nonzero inputs.
    pub fn set_angular_damping(&mut self, damping: f32) -> Result<(), PhysicsError> {
        supported_zero(damping)?;
        self.angular_damping = damping;
        Ok(())
    }

    /// Store zero sleep threshold without enabling automatic sleeping.
    /// # Errors
    /// Returns Numeric for nonfinite inputs, Unsupported for finite nonzero inputs.
    pub fn set_sleep_threshold(&mut self, threshold: f32) -> Result<(), PhysicsError> {
        supported_zero(threshold)?;
        self.sleep_threshold = threshold;
        Ok(())
    }

    /// Replace the typed supported policy; unsupported external enums need conversion.
    pub fn set_interpolation(&mut self, interpolation: BodyInterpolation) {
        self.interpolation = interpolation;
    }
}
