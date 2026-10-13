//! Independently designed finite 3D physics values and ownership contracts.
#![forbid(unsafe_code)]

mod body;
mod body_configuration;
mod body_profile;
mod collision_error;
mod gravity;
mod motion_error;
mod query;
mod simulation_mode;

pub use body::{BodyId, BodyState};
pub use body_configuration::BodyConfiguration;
pub use collision_error::CollisionError;
pub use gravity::{GravityConfiguration, GravityError};
pub use motion_error::PhysicsError;
pub use query::{QueryError, RaySegmentHit};
pub use simulation_mode::{BodyInterpolation, SimulationMode};
