//! Bounded ownership and configuration for Rapier 2D rigid bodies.

mod body_lifecycle;
mod configuration;
mod limits;
mod stepping;

pub use configuration::CollisionWorld2D;
pub use limits::{
    MAX_ABSOLUTE_GRAVITY_COMPONENT, MAX_ABSOLUTE_POSITION_COMPONENT, MAX_COLLISION_BODIES_2D,
    MAX_CUBOID_HALF_EXTENT, MAX_TIMESTEP_SECONDS, MIN_CUBOID_HALF_EXTENT,
};
