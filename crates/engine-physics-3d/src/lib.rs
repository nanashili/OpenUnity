//! One native-owned collider-free body and independently designed finite motion.
#![forbid(unsafe_code)]

mod manual_step;
mod world;

pub use manual_step::PreparedStep;
pub use world::MotionWorld;
