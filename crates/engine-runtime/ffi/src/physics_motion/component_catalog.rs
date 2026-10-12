//! Additive complete component-catalog transport for physics queries.

use crate::runtime_store::with_runtime;
use crate::status::{INVALID_KIND, object_id, runtime_error};
use engine_object::ObjectId;

/// Return the complete committed component count for a GameObject.
#[unsafe(no_mangle)]
pub extern "C" fn ou_physics_component_count(runtime: i64, owner: i64) -> i32 {
    with_runtime(runtime, |runtime| {
        object_id(owner)
            .and_then(|owner| runtime.physics_component_count(owner))
            .map_or_else(runtime_error, |count| {
                i32::try_from(count).unwrap_or(crate::status::COUNT_OVERFLOW)
            })
    })
    .unwrap_or_else(|error| error)
}

/// Return one complete-catalog component identity at a Transform-first index.
#[unsafe(no_mangle)]
pub extern "C" fn ou_physics_component_at(runtime: i64, owner: i64, index: i32) -> i64 {
    if index < 0 {
        return i64::from(INVALID_KIND);
    }

    let index = usize::try_from(index).expect("nonnegative ABI index fits usize");
    with_runtime(runtime, |runtime| {
        object_id(owner)
            .and_then(|owner| runtime.physics_component_at(owner, index))
            .map_or_else(|error| i64::from(runtime_error(error)), ObjectId::to_raw)
    })
    .unwrap_or_else(i64::from)
}
