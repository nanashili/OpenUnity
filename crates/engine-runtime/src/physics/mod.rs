//! Runtime composition boundary for the one-body motion slice.

mod admission;
mod api_lifetime;
mod api_properties;
mod attachment;
mod body_properties;
mod catalog_api;
mod manual_step;
mod retirement;
mod transform_sync;

pub(crate) use attachment::PhysicsState;
