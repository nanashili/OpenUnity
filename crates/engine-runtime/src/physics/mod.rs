//! Runtime composition boundary for motion state and owner-bound query state.

mod admission;
mod api_lifetime;
mod api_properties;
mod attachment;
mod attachment_registry;
mod body_properties;
mod catalog_api;
mod manual_step;
mod ordered_attachments;
mod owner_body_index;
mod query;
mod retirement;
mod retirement_plan;
mod transform_sync;

pub(crate) use attachment_registry::PhysicsState;
