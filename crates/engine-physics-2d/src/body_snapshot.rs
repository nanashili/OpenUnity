//! Owned finite values copied from a live 2D body.

use engine_physics_api::CollisionError;
use rapier2d::prelude::RigidBody;

/// Finite body state copied out of a 2D collision world.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BodySnapshot2D {
    position: [f32; 2],
    rotation_radians: f32,
    linear_velocity: [f32; 2],
    angular_velocity: f32,
    dynamic: bool,
}

impl BodySnapshot2D {
    /// Return the copied world-space position.
    pub fn position(self) -> [f32; 2] {
        self.position
    }

    /// Return the copied body rotation in radians.
    pub fn rotation_radians(self) -> f32 {
        self.rotation_radians
    }

    /// Return the copied world-space linear velocity.
    pub fn linear_velocity(self) -> [f32; 2] {
        self.linear_velocity
    }

    /// Return the copied angular velocity in radians per second.
    pub fn angular_velocity(self) -> f32 {
        self.angular_velocity
    }

    /// Whether the body is dynamic rather than fixed.
    pub fn is_dynamic(self) -> bool {
        self.dynamic
    }

    fn has_finite_values(self) -> bool {
        self.position
            .iter()
            .chain(self.linear_velocity.iter())
            .all(|value| value.is_finite())
            && self.rotation_radians.is_finite()
            && self.angular_velocity.is_finite()
    }
}

pub(crate) fn copy_from(body: &RigidBody) -> Result<BodySnapshot2D, CollisionError> {
    let translation = body.translation();
    let velocity = body.linvel();
    let snapshot = BodySnapshot2D {
        position: [translation.x, translation.y],
        rotation_radians: body.rotation().angle(),
        linear_velocity: [velocity.x, velocity.y],
        angular_velocity: body.angvel(),
        dynamic: body.is_dynamic(),
    };

    if snapshot.has_finite_values() {
        Ok(snapshot)
    } else {
        Err(CollisionError::InvalidNumber)
    }
}
