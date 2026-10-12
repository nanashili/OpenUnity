//! Owned, validated configuration of a collider-free awake body.

use crate::BodyInterpolation;
use engine_math::Vector;

/// Finite configuration values supported by the first 3D motion slice.
///
/// Private fields ensure that every public configuration is valid. Mutating a
/// copied configuration does not mutate a world; composition publishes it there.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BodyConfiguration {
    pub(crate) mass: f32,
    pub(crate) use_gravity: bool,
    pub(crate) angular_velocity: Vector,
    pub(crate) is_kinematic: bool,
    pub(crate) detect_collisions: bool,
    pub(crate) linear_damping: f32,
    pub(crate) angular_damping: f32,
    pub(crate) sleep_threshold: f32,
    pub(crate) interpolation: BodyInterpolation,
}

impl Default for BodyConfiguration {
    fn default() -> Self {
        Self {
            mass: 1.0,
            use_gravity: true,
            angular_velocity: [0.0; 3],
            is_kinematic: false,
            detect_collisions: false,
            linear_damping: 0.0,
            angular_damping: 0.0,
            sleep_threshold: 0.0,
            interpolation: BodyInterpolation::None,
        }
    }
}

impl BodyConfiguration {
    /// Stored positive finite mass; it does not affect force-free gravity motion.
    pub fn mass(self) -> f32 {
        self.mass
    }

    /// Whether a manual step applies the supplied gravity vector.
    pub fn use_gravity(self) -> bool {
        self.use_gravity
    }

    /// Stored zero angular velocity; rotation integration is unsupported.
    pub fn angular_velocity(self) -> Vector {
        self.angular_velocity
    }

    /// Stored dynamic-body flag; only false is supported.
    pub fn is_kinematic(self) -> bool {
        self.is_kinematic
    }

    /// Stored collision flag; only false is supported.
    pub fn detect_collisions(self) -> bool {
        self.detect_collisions
    }

    /// Stored linear damping; only zero is supported.
    pub fn linear_damping(self) -> f32 {
        self.linear_damping
    }

    /// Stored angular damping; only zero is supported.
    pub fn angular_damping(self) -> f32 {
        self.angular_damping
    }

    /// Stored sleep threshold; only zero is supported.
    pub fn sleep_threshold(self) -> f32 {
        self.sleep_threshold
    }

    /// Stored interpolation policy; only None is supported.
    pub fn interpolation(self) -> BodyInterpolation {
        self.interpolation
    }
}
