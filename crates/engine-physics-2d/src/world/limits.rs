//! Supported bounds for the 2D collision-world profile.

/// Maximum number of bodies admitted by one 2D collision world.
pub const MAX_COLLISION_BODIES_2D: usize = 256;

/// Maximum absolute value of each body position component, in meters.
pub const MAX_ABSOLUTE_POSITION_COMPONENT: f32 = 100_000.0;

/// Maximum absolute value of each gravity component, in meters per second squared.
pub const MAX_ABSOLUTE_GRAVITY_COMPONENT: f32 = 10_000.0;

/// Maximum half-extent of an inserted cuboid, in meters.
pub const MAX_CUBOID_HALF_EXTENT: f32 = 10_000.0;

/// Minimum half-extent that remains representable at every accepted position.
pub const MIN_CUBOID_HALF_EXTENT: f32 = 0.004;

/// Maximum caller-provided simulation timestep, in seconds.
pub const MAX_TIMESTEP_SECONDS: f32 = 0.25;
