//! Bounded 2D rigid-body collision simulation using the pinned Rapier backend.
//!
//! Each world supports at most 256 engine-owned bodies, dynamic and fixed box
//! colliders, and caller-timed simulation steps. Position components are capped
//! at 100,000 units, gravity at 10,000 units/s², half-extents from 0.004 to
//! 10,000 units, and timesteps at 0.25 seconds. This independent native slice
//! does not claim Unity Physics2D compatibility. Kinematic bodies, joints,
//! triggers, filters, collision events, queries, and Unity-specific behavior
//! are unsupported.
#![forbid(unsafe_code)]

mod body_snapshot;
mod validation;
mod world;

pub use body_snapshot::BodySnapshot2D;
pub use world::{
    CollisionWorld2D, MAX_ABSOLUTE_GRAVITY_COMPONENT, MAX_ABSOLUTE_POSITION_COMPONENT,
    MAX_COLLISION_BODIES_2D, MAX_CUBOID_HALF_EXTENT, MAX_TIMESTEP_SECONDS, MIN_CUBOID_HALF_EXTENT,
};
