//! Transform parent/child identity transport and atomic reparent requests.

use crate::runtime_store::with_runtime;
use crate::status::{INVALID_KIND, object_id, runtime_error};
use engine_object::ObjectId;

/// Version of the optional Transform hierarchy extension.
pub const TRANSFORM_HIERARCHY_ABI_VERSION: u32 = 0x0001_0000;

/// Return the optional Transform hierarchy ABI version.
// SAFETY: unique symbol within the reserved ABI namespace.
#[unsafe(no_mangle)]
pub extern "C" fn ou_transform_hierarchy_abi_version() -> u32 {
    TRANSFORM_HIERARCHY_ABI_VERSION
}

/// Return a Transform's parent identity, zero for root, or a negative error.
// SAFETY: unique symbol within the reserved ABI namespace.
#[unsafe(no_mangle)]
pub extern "C" fn ou_transform_parent(runtime: i64, transform: i64) -> i64 {
    with_runtime(runtime, |runtime| {
        object_id(transform)
            .and_then(|id| runtime.transform_parent(id))
            .map(|parent| parent.map_or(0, ObjectId::to_raw))
            .unwrap_or_else(|error| i64::from(runtime_error(error)))
    })
    .unwrap_or_else(i64::from)
}

/// Return a Transform's ordered child count or a negative error.
// SAFETY: unique symbol within the reserved ABI namespace.
#[unsafe(no_mangle)]
pub extern "C" fn ou_transform_child_count(runtime: i64, transform: i64) -> i32 {
    with_runtime(runtime, |runtime| {
        object_id(transform)
            .and_then(|id| runtime.transform_child_count(id))
            .unwrap_or_else(runtime_error)
    })
    .unwrap_or_else(|error| error)
}

/// Return an ordered child identity or a negative error.
// SAFETY: unique symbol within the reserved ABI namespace.
#[unsafe(no_mangle)]
pub extern "C" fn ou_transform_child(runtime: i64, transform: i64, index: i32) -> i64 {
    with_runtime(runtime, |runtime| {
        object_id(transform)
            .and_then(|id| runtime.transform_child(id, index))
            .map(ObjectId::to_raw)
            .unwrap_or_else(|error| i64::from(runtime_error(error)))
    })
    .unwrap_or_else(i64::from)
}

/// Atomically change a parent with an explicit zero/one world preservation flag.
// SAFETY: unique symbol within the reserved ABI namespace.
#[unsafe(no_mangle)]
pub extern "C" fn ou_transform_set_parent(
    runtime: i64,
    transform: i64,
    parent: i64,
    world_position_stays: i32,
) -> i32 {
    if !matches!(world_position_stays, 0 | 1) {
        return INVALID_KIND;
    }
    with_runtime(runtime, |runtime| {
        let mut operation = || {
            let id = object_id(transform)?;
            let parent = if parent == 0 {
                None
            } else {
                Some(object_id(parent)?)
            };
            runtime.transform_set_parent(id, parent, world_position_stays == 1)
        };
        operation().map_or_else(runtime_error, |()| 0)
    })
    .unwrap_or_else(|error| error)
}

/// Reorder a Transform within its existing direct parent's child sequence.
// SAFETY: unique symbol within the reserved ABI namespace.
#[unsafe(no_mangle)]
pub extern "C" fn ou_transform_set_sibling_index(runtime: i64, transform: i64, index: i32) -> i32 {
    with_runtime(runtime, |runtime| {
        object_id(transform)
            .and_then(|id| runtime.transform_set_sibling_index(id, index))
            .map_or_else(runtime_error, |()| 0)
    })
    .unwrap_or_else(|error| error)
}
