//! Bounded 3D cuboid collision simulation backed by Rapier's real solver.

mod solver;

use engine_math::{Quaternion, Vector, finite, normalize};
use engine_physics_api::{BodyId, CollisionError};
use rapier3d::prelude::RigidBodyBuilder;

use self::solver::SolverState;

/// Maximum dynamic and fixed cuboids in one collision world.
pub const MAX_COLLISION_BODIES: usize = 128;

/// Maximum magnitude of any world-space position component, in engine units.
pub const MAX_ABSOLUTE_POSITION_COMPONENT: f32 = 100_000.0;

/// Maximum magnitude of any gravity component, in engine units per second squared.
pub const MAX_ABSOLUTE_GRAVITY_COMPONENT: f32 = 10_000.0;

/// Maximum cuboid half-extent component, in engine units.
pub const MAX_CUBOID_HALF_EXTENT: f32 = 10_000.0;

/// Minimum cuboid half-extent component, in engine units.
///
/// The maximum supported position has an f32 spacing of 0.0078125 units, so
/// this value stays above half that spacing and keeps cuboid bounds distinct.
/// It also avoids degenerate dynamic mass data for very small cuboids.
pub const MIN_CUBOID_HALF_EXTENT: f32 = 0.004;

/// Maximum accepted caller-supplied simulation step, in seconds.
pub const MAX_TIMESTEP_SECONDS: f32 = 0.25;

/// A finite copied pose and linear velocity from a collision body.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CollisionBodySnapshot {
    position: Vector,
    rotation: Quaternion,
    linear_velocity: Vector,
}

impl CollisionBodySnapshot {
    /// Return the copied world-space position.
    pub fn position(self) -> Vector {
        self.position
    }

    /// Return the copied normalized x/y/z/w rotation.
    pub fn rotation(self) -> Quaternion {
        self.rotation
    }

    /// Return the copied world-space linear velocity.
    pub fn linear_velocity(self) -> Vector {
        self.linear_velocity
    }
}

/// A bounded cuboid world with explicit gravity and caller-controlled steps.
///
/// Position components are limited to ±100,000 units, gravity components to
/// ±10,000 units/s², half-extents to [0.004, 10,000] units, and steps to
/// (0, 0.25] s.
/// It supports dynamic and fixed cuboids, gravity, and contact response. It has
/// no events, filters, sensors, joints, kinematic bodies, angular-velocity
/// controls, or Unity-specific material behavior.
#[derive(Clone)]
pub struct CollisionWorld3D {
    gravity: Vector,
    solver: SolverState,
}

impl CollisionWorld3D {
    /// Create an empty world with bounded gravity supplied by the caller.
    ///
    /// # Errors
    /// Returns [`CollisionError::InvalidNumber`] for nonfinite or out-of-range gravity.
    pub fn new(gravity: Vector) -> Result<Self, CollisionError> {
        validate_bounded_vector(gravity, MAX_ABSOLUTE_GRAVITY_COMPONENT)?;
        Ok(Self {
            gravity,
            solver: SolverState::default(),
        })
    }

    /// Replace bounded gravity without changing simulated bodies.
    ///
    /// # Errors
    /// Returns [`CollisionError::InvalidNumber`] for nonfinite or out-of-range gravity.
    pub fn set_gravity(&mut self, gravity: Vector) -> Result<(), CollisionError> {
        validate_bounded_vector(gravity, MAX_ABSOLUTE_GRAVITY_COMPONENT)?;
        self.gravity = gravity;
        Ok(())
    }

    /// Add a dynamic cuboid with zero initial linear velocity.
    ///
    /// # Errors
    /// Returns a collision error for an invalid or out-of-range pose/dimension,
    /// duplicate ID, or capacity exhaustion.
    pub fn insert_dynamic_cuboid(
        &mut self,
        id: BodyId,
        position: Vector,
        rotation: Quaternion,
        half_extents: Vector,
    ) -> Result<(), CollisionError> {
        self.insert_cuboid(
            RigidBodyBuilder::dynamic(),
            id,
            position,
            rotation,
            half_extents,
        )
    }

    /// Add a fixed cuboid with the supplied initial pose.
    ///
    /// # Errors
    /// Returns a collision error for an invalid or out-of-range pose/dimension,
    /// duplicate ID, or capacity exhaustion.
    pub fn insert_fixed_cuboid(
        &mut self,
        id: BodyId,
        position: Vector,
        rotation: Quaternion,
        half_extents: Vector,
    ) -> Result<(), CollisionError> {
        self.insert_cuboid(
            RigidBodyBuilder::fixed(),
            id,
            position,
            rotation,
            half_extents,
        )
    }

    /// Return the finite copied pose and velocity for an owned body ID.
    ///
    /// # Errors
    /// Returns [`CollisionError::InvalidBody`] or [`CollisionError::InvalidNumber`].
    pub fn body(&self, id: BodyId) -> Result<CollisionBodySnapshot, CollisionError> {
        self.solver.body(id)
    }

    /// Remove one body and its attached collider deterministically.
    ///
    /// # Errors
    /// Returns [`CollisionError::InvalidBody`] if the ID is not owned here.
    pub fn remove(&mut self, id: BodyId) -> Result<(), CollisionError> {
        self.solver.remove(id)
    }

    /// Run one bounded solver step, publishing only supported body snapshots.
    ///
    /// # Errors
    /// Returns [`CollisionError::InvalidTimestep`] for a nonpositive, nonfinite,
    /// or greater-than-maximum timestep, or [`CollisionError::InvalidNumber`]
    /// for an out-of-range solver result.
    pub fn step(&mut self, timestep: f32) -> Result<(), CollisionError> {
        validate_timestep(timestep)?;
        self.solver.step(self.gravity, timestep)
    }

    /// Return the number of dynamic and fixed cuboids in this world.
    pub fn body_count(&self) -> usize {
        self.solver.body_count()
    }

    fn insert_cuboid(
        &mut self,
        builder: RigidBodyBuilder,
        id: BodyId,
        position: Vector,
        rotation: Quaternion,
        half_extents: Vector,
    ) -> Result<(), CollisionError> {
        validate_bounded_vector(position, MAX_ABSOLUTE_POSITION_COMPONENT)?;
        validate_dimensions(half_extents)?;
        let rotation = normalize(rotation).map_err(|_| CollisionError::InvalidNumber)?;
        self.solver
            .add_cuboid(id, builder, position, rotation, half_extents)
    }
}

fn validate_bounded_vector(value: Vector, maximum_magnitude: f32) -> Result<(), CollisionError> {
    finite(&value).map_err(|_| CollisionError::InvalidNumber)?;

    if value
        .iter()
        .all(|component| component.abs() <= maximum_magnitude)
    {
        Ok(())
    } else {
        Err(CollisionError::InvalidNumber)
    }
}

fn validate_dimensions(half_extents: Vector) -> Result<(), CollisionError> {
    if half_extents.iter().all(|value| {
        value.is_finite() && *value >= MIN_CUBOID_HALF_EXTENT && *value <= MAX_CUBOID_HALF_EXTENT
    }) {
        Ok(())
    } else {
        Err(CollisionError::InvalidDimensions)
    }
}

fn validate_timestep(timestep: f32) -> Result<(), CollisionError> {
    if timestep.is_finite() && timestep > 0.0 && timestep <= MAX_TIMESTEP_SECONDS {
        Ok(())
    } else {
        Err(CollisionError::InvalidTimestep)
    }
}
