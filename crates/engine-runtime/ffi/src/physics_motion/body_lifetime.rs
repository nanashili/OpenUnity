//! Additive native transport for committed Rigidbody lifetime.

use crate::runtime_store::with_runtime;
use crate::status::{object_id, runtime_error};
use engine_object::ObjectId;

/// Motion capability major 1, minor 0.
pub const PHYSICS_MOTION_ABI_VERSION: u32 = 0x0001_0000;

/// Return the additive motion capability without accessing runtime state.
#[unsafe(no_mangle)]
pub extern "C" fn ou_physics_motion_abi_version() -> u32 {
    PHYSICS_MOTION_ABI_VERSION
}

/// Reserve one hidden Rigidbody component for a GameObject.
#[unsafe(no_mangle)]
pub extern "C" fn ou_rigidbody_reserve(runtime: i64, owner: i64) -> i64 {
    with_runtime(runtime, |runtime| {
        object_id(owner)
            .and_then(|owner| runtime.rigidbody_reserve(owner))
            .map_or_else(|error| i64::from(runtime_error(error)), ObjectId::to_raw)
    })
    .unwrap_or_else(i64::from)
}

/// Publish a previously reserved Rigidbody component.
#[unsafe(no_mangle)]
pub extern "C" fn ou_rigidbody_commit(runtime: i64, body: i64) -> i32 {
    with_runtime(runtime, |runtime| {
        object_id(body)
            .and_then(|body| runtime.rigidbody_commit(body))
            .map_or_else(runtime_error, |_| 0)
    })
    .unwrap_or_else(|error| error)
}

/// Abort only an unpublished Rigidbody reservation.
#[unsafe(no_mangle)]
pub extern "C" fn ou_rigidbody_abort(runtime: i64, body: i64) -> i32 {
    with_runtime(runtime, |runtime| {
        object_id(body)
            .and_then(|body| runtime.rigidbody_abort(body))
            .map_or_else(runtime_error, |_| 0)
    })
    .unwrap_or_else(|error| error)
}
