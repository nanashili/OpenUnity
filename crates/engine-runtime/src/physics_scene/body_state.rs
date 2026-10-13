use crate::math::{Quaternion, Vector};

/// Collision profile selected for one runtime-owned scene.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PhysicsSceneDimension {
    /// A planar scene whose poses occupy the runtime XY plane.
    TwoD,
    /// A spatial scene with full 3D poses.
    ThreeD,
}

/// An owned pose and velocity copy from one admitted scene body.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PhysicsSceneBodyState {
    position: Vector,
    rotation: Quaternion,
    linear_velocity: Vector,
    dynamic: bool,
}

impl PhysicsSceneBodyState {
    pub(crate) const fn new(
        position: Vector,
        rotation: Quaternion,
        linear_velocity: Vector,
        dynamic: bool,
    ) -> Self {
        Self {
            position,
            rotation,
            linear_velocity,
            dynamic,
        }
    }

    /// Return the last published runtime position.
    pub const fn position(self) -> Vector {
        self.position
    }

    /// Return the normalized x/y/z/w runtime rotation.
    pub const fn rotation(self) -> Quaternion {
        self.rotation
    }

    /// Return the copied linear velocity.
    pub const fn linear_velocity(self) -> Vector {
        self.linear_velocity
    }

    /// Whether the registered body participates as a dynamic body.
    pub const fn is_dynamic(self) -> bool {
        self.dynamic
    }
}
