//! One native-owned collider-free body and independently designed finite motion.
#![forbid(unsafe_code)]

mod manual_step;
mod query_world;
mod world;

pub use manual_step::PreparedStep;
pub use query_world::{AabbQueryWorld, MAX_QUERY_BOXES, PreparedOwnerRetirement};
pub use world::MotionWorld;
