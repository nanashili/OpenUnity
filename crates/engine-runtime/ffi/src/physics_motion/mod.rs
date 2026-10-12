//! FFI transport for the additive one-body motion capability.

mod body_lifetime;
mod body_properties;
mod component_catalog;
mod manual_step;
mod vector_buffer;

pub use body_lifetime::{
    PHYSICS_MOTION_ABI_VERSION, ou_physics_motion_abi_version, ou_rigidbody_abort,
    ou_rigidbody_commit, ou_rigidbody_reserve,
};
pub use body_properties::{
    ou_rigidbody_flag_read, ou_rigidbody_flag_write, ou_rigidbody_interpolation_read,
    ou_rigidbody_interpolation_write, ou_rigidbody_scalar_read, ou_rigidbody_scalar_write,
    ou_rigidbody_wake_up,
};
pub use component_catalog::{ou_physics_component_at, ou_physics_component_count};
pub use manual_step::{
    ou_physics_mode_read, ou_physics_mode_write, ou_physics_simulate, ou_physics_sync_transforms,
};
pub use vector_buffer::{ou_rigidbody_vector_read, ou_rigidbody_vector_write};
