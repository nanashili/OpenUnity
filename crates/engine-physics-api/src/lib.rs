//! Independently designed 3D physics configuration without a simulation backend.
#![forbid(unsafe_code)]

mod gravity;

pub use gravity::{GravityConfiguration, GravityError};
