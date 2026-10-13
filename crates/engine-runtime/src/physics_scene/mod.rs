//! Runtime-owned collision-scene lifecycle, body admission and pose publication.

mod admission;
mod body_admission;
mod body_removal;
mod body_state;
mod retirement;
mod scene_state;
mod selection;
mod stepping;

pub use body_state::{PhysicsSceneBodyState, PhysicsSceneDimension};
pub(crate) use scene_state::CollisionSceneState;
