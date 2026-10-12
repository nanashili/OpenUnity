//! Owned object identity creation, liveness, destruction and kind queries.

use crate::runtime_store::with_runtime;
use crate::status::{IDENTITY_EXHAUSTED, INVALID_OBJECT, object_id, runtime_error};
use engine_object::ObjectId;

/// Allocate an identity in this context, returning a positive handle or negative error.
// SAFETY: unique symbol in the reserved ABI namespace; fixed-width integer interface.
#[unsafe(no_mangle)]
pub extern "C" fn ou_object_create(runtime: i64) -> i64 {
    with_runtime(runtime, |runtime| {
        runtime
            .create_object()
            .map(ObjectId::to_raw)
            .unwrap_or(i64::from(IDENTITY_EXHAUSTED))
    })
    .unwrap_or_else(i64::from)
}

/// Return 1 for live owned identities, 0 for other objects, or a negative context error.
// SAFETY: unique symbol in the reserved ABI namespace; fixed-width integer interface.
#[unsafe(no_mangle)]
pub extern "C" fn ou_object_is_alive(runtime: i64, object: i64) -> i32 {
    with_runtime(runtime, |runtime| {
        i32::from(ObjectId::from_raw(object).is_some_and(|id| runtime.is_alive(id)))
    })
    .unwrap_or_else(|error| error)
}

/// Destroy an owned identity, returning zero or a negative error code.
// SAFETY: unique symbol in the reserved ABI namespace; fixed-width integer interface.
#[unsafe(no_mangle)]
pub extern "C" fn ou_object_destroy(runtime: i64, object: i64) -> i32 {
    with_runtime(runtime, |runtime| {
        let Some(id) = ObjectId::from_raw(object) else {
            return INVALID_OBJECT;
        };
        runtime
            .destroy_typed_object(id)
            .map_or_else(runtime_error, |()| 0)
    })
    .unwrap_or_else(|error| error)
}

/// Return plain/GameObject/Transform kind 1/2/3, or a negative error.
// SAFETY: unique symbol within the reserved ABI namespace.
#[unsafe(no_mangle)]
pub extern "C" fn ou_object_kind(runtime: i64, object: i64) -> i32 {
    with_runtime(runtime, |runtime| {
        object_id(object)
            .and_then(|id| runtime.object_kind(id))
            .unwrap_or_else(runtime_error)
    })
    .unwrap_or_else(|error| error)
}
