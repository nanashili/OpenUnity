//! OpenUnity C ABI v1 with an independently versioned owned UTF-8 name extension.
//! No Unity assemblies or runtime are loaded.
//!
//! All exports use the reserved `ou_` prefix. Name buffers are copied during the
//! call under the runtime-store mutex; no borrowed storage pointer escapes.
//! Unexpected internal panics and allocation failure terminate the process;
//! ordinary invalid input, identity exhaustion and poisoned storage return errors.

mod abi;
mod lifecycle;
mod object_lifetime;
mod object_name;
mod object_ownership;
mod physics_gravity;
mod physics_motion;
mod physics_query;
mod runtime_store;
mod scene;
mod scene_exports;
mod scene_roots;
mod scene_snapshot;
mod scene_validity;
mod status;
mod timing;
mod transform_detach;
mod transform_direction;
mod transform_hierarchy;
mod transform_pose;
mod transform_vector;

pub use abi::{
    ABI_VERSION, OBJECT_NAME_ABI_VERSION, TRANSFORM_ABI_VERSION, ou_abi_version,
    ou_object_name_abi_version, ou_transform_abi_version,
};
pub use lifecycle::{
    LIFECYCLE_ABI_VERSION, ou_behaviour_active, ou_behaviour_enabled, ou_behaviour_set_enabled,
    ou_component_abort, ou_component_commit, ou_component_reserve, ou_gameobject_active,
    ou_gameobject_component_at, ou_gameobject_component_count, ou_gameobject_set_active,
    ou_lifecycle_abi_version, ou_lifecycle_ack, ou_lifecycle_begin_frame, ou_lifecycle_begin_phase,
    ou_lifecycle_flush_destroy, ou_lifecycle_frame_count, ou_lifecycle_next,
    ou_object_destroy_deferred,
};
pub use object_lifetime::{
    ou_object_create, ou_object_destroy, ou_object_is_alive, ou_object_kind,
};
pub use object_name::{ou_object_name_len, ou_object_name_read, ou_object_name_set};
pub use object_ownership::{
    ou_component_gameobject, ou_gameobject_create, ou_gameobject_transform,
};
pub use physics_gravity::{
    PHYSICS_GRAVITY_ABI_VERSION, ou_physics_gravity_abi_version, ou_physics_gravity_read,
    ou_physics_gravity_write,
};
pub use physics_motion::{
    PHYSICS_MOTION_ABI_VERSION, ou_physics_component_at, ou_physics_component_count,
    ou_physics_mode_read, ou_physics_mode_write, ou_physics_motion_abi_version,
    ou_physics_simulate, ou_physics_sync_transforms, ou_rigidbody_abort, ou_rigidbody_commit,
    ou_rigidbody_flag_read, ou_rigidbody_flag_write, ou_rigidbody_interpolation_read,
    ou_rigidbody_interpolation_write, ou_rigidbody_reserve, ou_rigidbody_scalar_read,
    ou_rigidbody_scalar_write, ou_rigidbody_vector_read, ou_rigidbody_vector_write,
    ou_rigidbody_wake_up,
};
pub use physics_query::{
    PHYSICS_QUERY_ABI_VERSION, ou_physics_aabb_add, ou_physics_overlap_aabb,
    ou_physics_query_abi_version, ou_physics_raycast_segment,
};
pub use runtime_store::{ou_runtime_create, ou_runtime_destroy};
pub use scene_exports::*;
pub use scene_roots::*;
pub use scene_snapshot::{
    SCENE_SNAPSHOT_ABI_VERSION, ou_scene_snapshot, ou_scene_snapshot_abi_version,
};
pub use status::{
    ALLOCATION_FAILURE, COUNT_OVERFLOW, GAMEPLAY_FAULTED, HIERARCHY_CYCLE, IDENTITY_EXHAUSTED,
    INSUFFICIENT_CAPACITY, INVALID_BUFFER, INVALID_KIND, INVALID_LIFECYCLE, INVALID_NUMERIC,
    INVALID_OBJECT, INVALID_PHYSICS_QUERY, INVALID_RUNTIME, INVALID_SCENE, INVALID_SCENE_PLAN,
    INVALID_SIBLING_INDEX, INVALID_UTF8, PHYSICS_UNSUPPORTED, STAGING_BUSY, STORE_UNAVAILABLE,
    UNSUPPORTED_TRANSFORM,
};
pub use timing::{
    TIME_ABI_VERSION, ou_time_abi_version, ou_time_begin_fixed, ou_time_begin_frame,
    ou_time_configure, ou_time_next, ou_time_set, ou_time_snapshot,
};
pub use transform_detach::{
    TRANSFORM_DETACH_ABI_VERSION, ou_transform_detach_abi_version, ou_transform_detach_children,
};
pub use transform_direction::{
    TRANSFORM_DIRECTION_ABI_VERSION, ou_transform_direction, ou_transform_direction_abi_version,
};
pub use transform_hierarchy::{
    TRANSFORM_HIERARCHY_ABI_VERSION, ou_transform_child, ou_transform_child_count,
    ou_transform_hierarchy_abi_version, ou_transform_parent, ou_transform_set_as_last_sibling,
    ou_transform_set_parent, ou_transform_set_sibling_index,
};
pub use transform_pose::{ou_transform_point, ou_transform_read, ou_transform_write};
pub use transform_vector::{
    TRANSFORM_VECTOR_ABI_VERSION, ou_transform_vector, ou_transform_vector_abi_version,
};
