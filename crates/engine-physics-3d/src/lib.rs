//! Native-owned collider-free motion bodies and independently designed finite motion.
#![forbid(unsafe_code)]

mod body_registry;
mod collision_world;
mod manual_step;
mod query_world;
mod world;

pub use body_registry::MAX_MOTION_BODIES;
pub use collision_world::{
    CollisionBodySnapshot, CollisionWorld3D, MAX_ABSOLUTE_GRAVITY_COMPONENT,
    MAX_ABSOLUTE_POSITION_COMPONENT, MAX_COLLISION_BODIES, MAX_CUBOID_HALF_EXTENT,
    MAX_TIMESTEP_SECONDS, MIN_CUBOID_HALF_EXTENT,
};
pub use manual_step::PreparedStep;
pub use query_world::{AabbQueryWorld, MAX_QUERY_BOXES, PreparedOwnerRetirement};
pub use world::MotionWorld;
