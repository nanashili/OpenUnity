//! Runtime composition boundary for motion state and owner-bound query state.

mod admission;
mod api_lifetime;
mod api_properties;
mod attachment;
mod body_properties;
mod catalog_api;
mod manual_step;
mod query;
mod retirement;
mod transform_sync;

pub(crate) use attachment::PhysicsState;
