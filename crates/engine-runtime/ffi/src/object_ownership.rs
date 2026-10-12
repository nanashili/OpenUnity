//! Typed GameObject creation and mandatory component-owner identity access.

use crate::object_name::buffer_len;
use crate::runtime_store::with_runtime;
use crate::status::{IDENTITY_EXHAUSTED, INVALID_UTF8, object_id, runtime_error};
use engine_object::ObjectId;

/// Atomically create a GameObject/Transform pair with an owned strict UTF-8 name.
/// Pointer/count validation precedes runtime lookup, then UTF-8 validation.
/// # Safety
/// Nonempty input must provide length initialized readable bytes throughout the
/// call, without conflicting concurrent access. Null is permitted for length zero.
// SAFETY: unique symbol within the reserved ABI namespace.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ou_gameobject_create(runtime: i64, name: *const u8, length: i32) -> i64 {
    let length = match buffer_len(name, length) {
        Ok(length) => length,
        Err(error) => return i64::from(error),
    };
    with_runtime(runtime, |runtime| {
        let bytes = if length == 0 {
            &[]
        } else {
            // SAFETY: validated non-null pointer and bounded byte count; caller
            // guarantees the initialized readable region for the complete call.
            unsafe { std::slice::from_raw_parts(name, length) }
        };
        let Ok(name) = std::str::from_utf8(bytes) else {
            return i64::from(INVALID_UTF8);
        };
        runtime
            .create_gameobject(name)
            .map(ObjectId::to_raw)
            .unwrap_or(i64::from(IDENTITY_EXHAUSTED))
    })
    .unwrap_or_else(i64::from)
}

/// Resolve a GameObject's mandatory Transform identity.
// SAFETY: unique symbol within the reserved ABI namespace.
#[unsafe(no_mangle)]
pub extern "C" fn ou_gameobject_transform(runtime: i64, gameobject: i64) -> i64 {
    with_runtime(runtime, |runtime| {
        object_id(gameobject)
            .and_then(|id| runtime.gameobject_transform(id))
            .map(ObjectId::to_raw)
            .unwrap_or_else(|error| i64::from(runtime_error(error)))
    })
    .unwrap_or_else(i64::from)
}

/// Resolve the owning GameObject of the supported Component kind.
// SAFETY: unique symbol within the reserved ABI namespace.
#[unsafe(no_mangle)]
pub extern "C" fn ou_component_gameobject(runtime: i64, component: i64) -> i64 {
    with_runtime(runtime, |runtime| {
        object_id(component)
            .and_then(|id| runtime.component_gameobject(id))
            .map(ObjectId::to_raw)
            .unwrap_or_else(|error| i64::from(runtime_error(error)))
    })
    .unwrap_or_else(i64::from)
}
