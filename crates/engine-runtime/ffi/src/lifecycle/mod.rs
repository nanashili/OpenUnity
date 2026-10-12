//! Integer-only lifecycle capability. Managed dispatch happens after exports return.

mod activity;
mod components;
mod delivery;
mod frames;
mod protocol;

pub use activity::{
    ou_behaviour_active, ou_behaviour_enabled, ou_behaviour_set_enabled, ou_gameobject_active,
    ou_gameobject_set_active,
};
pub use components::{
    ou_component_abort, ou_component_commit, ou_component_reserve, ou_gameobject_component_at,
    ou_gameobject_component_count, ou_object_destroy_deferred,
};
pub use delivery::{ou_lifecycle_ack, ou_lifecycle_next};
pub use frames::{
    ou_lifecycle_begin_frame, ou_lifecycle_begin_phase, ou_lifecycle_flush_destroy,
    ou_lifecycle_frame_count,
};
pub use protocol::{LIFECYCLE_ABI_VERSION, ou_lifecycle_abi_version};
