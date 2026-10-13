//! Native-owned collider-free motion bodies and independently designed finite motion.
#![forbid(unsafe_code)]

mod body_registry;
mod manual_step;
mod query_world;
mod world;

pub use body_registry::MAX_MOTION_BODIES;
pub use manual_step::PreparedStep;
pub use query_world::{AabbQueryWorld, MAX_QUERY_BOXES, PreparedOwnerRetirement};
pub use world::MotionWorld;
