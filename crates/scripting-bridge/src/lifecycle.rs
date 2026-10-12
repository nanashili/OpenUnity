//! Integer-only lifecycle capability. All managed dispatch happens after exports return.

use super::*;

/// Independent lifecycle extension major 1, minor 0.
pub const LIFECYCLE_ABI_VERSION: u32 = 0x0001_0000;
/// Invalid lifecycle phase, attachment state, or acknowledgement token.
pub const INVALID_LIFECYCLE: i32 = -13;
/// Gameplay callbacks failed and dispatch cannot resume.
pub const GAMEPLAY_FAULTED: i32 = -14;

fn status(result: Result<(), RuntimeError>) -> i32 {
    result.map_or_else(runtime_error, |()| 0)
}

fn flag(value: i32) -> Result<bool, i32> {
    match value {
        0 => Ok(false),
        1 => Ok(true),
        _ => Err(INVALID_KIND),
    }
}

/// Return the additive lifecycle capability without touching runtime storage.
// SAFETY: unique reserved export with fixed-width integer representation.
#[unsafe(no_mangle)]
pub extern "C" fn ou_lifecycle_abi_version() -> u32 {
    LIFECYCLE_ABI_VERSION
}

/// Reserve a hidden custom MonoBehaviour before managed construction.
// SAFETY: unique reserved export; only fixed-width integers cross the boundary.
#[unsafe(no_mangle)]
pub extern "C" fn ou_component_reserve(
    runtime: i64,
    owner: i64,
    kind: i32,
    execution_order: i32,
) -> i64 {
    if kind != 4 {
        return i64::from(INVALID_KIND);
    }
    with_runtime(runtime, |runtime| {
        object_id(owner)
            .and_then(|owner| runtime.component_reserve(owner, execution_order))
            .map_or_else(|error| i64::from(runtime_error(error)), ObjectId::to_raw)
    })
    .unwrap_or_else(i64::from)
}

macro_rules! object_mutation {
    ($name:ident, $method:ident, $doc:literal) => {
        #[doc = $doc]
        // SAFETY: unique reserved export with fixed-width integer interface.
        #[unsafe(no_mangle)]
        pub extern "C" fn $name(runtime: i64, object: i64) -> i32 {
            with_runtime(runtime, |runtime| {
                status(object_id(object).and_then(|id| runtime.$method(id)))
            })
            .unwrap_or_else(|error| error)
        }
    };
}

object_mutation!(
    ou_component_commit,
    component_commit,
    "Publish a constructed reservation atomically."
);
object_mutation!(
    ou_component_abort,
    component_abort,
    "Consume an unpublished reservation after construction fails."
);
object_mutation!(
    ou_object_destroy_deferred,
    object_destroy_deferred,
    "Queue destruction, preserving identities until the destruction phase completes."
);

/// Return Transform-first committed component count.
// SAFETY: unique reserved export with fixed-width integer interface.
#[unsafe(no_mangle)]
pub extern "C" fn ou_gameobject_component_count(runtime: i64, owner: i64) -> i32 {
    with_runtime(runtime, |runtime| {
        object_id(owner)
            .and_then(|id| runtime.gameobject_component_count(id))
            .unwrap_or_else(runtime_error)
    })
    .unwrap_or_else(|error| error)
}

/// Return a committed component ID at a Transform-first index.
// SAFETY: unique reserved export with fixed-width integer interface.
#[unsafe(no_mangle)]
pub extern "C" fn ou_gameobject_component_at(runtime: i64, owner: i64, index: i32) -> i64 {
    if index < 0 {
        return i64::from(INVALID_KIND);
    }
    with_runtime(runtime, |runtime| {
        object_id(owner)
            .and_then(|id| runtime.gameobject_component_at(id, index))
            .map_or_else(|error| i64::from(runtime_error(error)), ObjectId::to_raw)
    })
    .unwrap_or_else(i64::from)
}

/// Query activeSelf (hierarchy=0) or activeInHierarchy (hierarchy=1).
// SAFETY: unique reserved export with fixed-width integer interface.
#[unsafe(no_mangle)]
pub extern "C" fn ou_gameobject_active(runtime: i64, owner: i64, hierarchy: i32) -> i32 {
    let hierarchy = match flag(hierarchy) {
        Ok(value) => value,
        Err(error) => return error,
    };
    with_runtime(runtime, |runtime| {
        object_id(owner)
            .and_then(|id| runtime.gameobject_active(id, hierarchy))
            .map_or_else(runtime_error, i32::from)
    })
    .unwrap_or_else(|error| error)
}

macro_rules! flag_mutation {
    ($name:ident, $method:ident, $doc:literal) => {
        #[doc = $doc]
        // SAFETY: unique reserved export with fixed-width integer interface.
        #[unsafe(no_mangle)]
        pub extern "C" fn $name(runtime: i64, object: i64, value: i32) -> i32 {
            let value = match flag(value) {
                Ok(value) => value,
                Err(error) => return error,
            };
            with_runtime(runtime, |runtime| {
                status(object_id(object).and_then(|id| runtime.$method(id, value)))
            })
            .unwrap_or_else(|error| error)
        }
    };
}

flag_mutation!(
    ou_gameobject_set_active,
    gameobject_set_active,
    "Set owner activity before a separate mutation callback pump."
);
flag_mutation!(
    ou_behaviour_set_enabled,
    behaviour_set_enabled,
    "Set script enabled state before a separate mutation callback pump."
);

macro_rules! behaviour_query {
    ($name:ident, $method:ident, $doc:literal) => {
        #[doc = $doc]
        // SAFETY: unique reserved export with fixed-width integer interface.
        #[unsafe(no_mangle)]
        pub extern "C" fn $name(runtime: i64, component: i64) -> i32 {
            with_runtime(runtime, |runtime| {
                object_id(component)
                    .and_then(|id| runtime.$method(id))
                    .map_or_else(runtime_error, i32::from)
            })
            .unwrap_or_else(|error| error)
        }
    };
}

behaviour_query!(
    ou_behaviour_enabled,
    behaviour_enabled,
    "Read the committed script's native enabled flag."
);
behaviour_query!(
    ou_behaviour_active,
    behaviour_active,
    "Read enabled and effective native hierarchy activity."
);

/// Read the actual per-runtime native frame counter.
// SAFETY: unique reserved export with fixed-width integer interface.
#[unsafe(no_mangle)]
pub extern "C" fn ou_lifecycle_frame_count(runtime: i64) -> i32 {
    with_runtime(runtime, |runtime| runtime.lifecycle_frame_count()).unwrap_or_else(|error| error)
}

macro_rules! frame_mutation {
    ($name:ident, $method:ident, $doc:literal) => {
        #[doc = $doc]
        // SAFETY: unique reserved export with fixed-width integer interface.
        #[unsafe(no_mangle)]
        pub extern "C" fn $name(runtime: i64) -> i32 {
            with_runtime(runtime, |runtime| status(runtime.$method())).unwrap_or_else(|error| error)
        }
    };
}

frame_mutation!(
    ou_lifecycle_begin_frame,
    lifecycle_begin_frame,
    "Begin one frame, rejecting unfinished or recursive scheduling."
);
frame_mutation!(
    ou_lifecycle_flush_destroy,
    lifecycle_flush_destroy,
    "Enter a destruction-only boundary without changing the frame counter."
);

/// Begin the next ordered native frame phase, 1 through 5.
// SAFETY: unique reserved export with fixed-width integer interface.
#[unsafe(no_mangle)]
pub extern "C" fn ou_lifecycle_begin_phase(runtime: i64, phase: i32) -> i32 {
    if !(1..=5).contains(&phase) {
        return INVALID_KIND;
    }
    with_runtime(runtime, |runtime| {
        status(runtime.lifecycle_begin_phase(phase))
    })
    .unwrap_or_else(|error| error)
}

/// Acknowledge the current top callback token exactly once.
// SAFETY: unique reserved export with fixed-width integer interface.
#[unsafe(no_mangle)]
pub extern "C" fn ou_lifecycle_ack(runtime: i64, token: i64, callback_status: i32) -> i32 {
    if callback_status != 0 && callback_status != -6 {
        return INVALID_LIFECYCLE;
    }
    with_runtime(runtime, |runtime| {
        status(runtime.lifecycle_ack(token, callback_status))
    })
    .unwrap_or_else(|error| error)
}

fn output_range<T>(pointer: *mut T) -> Result<(usize, usize), i32> {
    if pointer.is_null() || !pointer.is_aligned() {
        return Err(INVALID_BUFFER);
    }
    let start = pointer.addr();
    let end = start
        .checked_add(std::mem::size_of::<T>())
        .ok_or(INVALID_BUFFER)?;
    Ok((start, end))
}

/// Copy one native delivery to caller-owned output storage.
///
/// On zero or failure every output is unchanged. Mode 0 returns mutation events;
/// mode 1 also advances the active frame/destruction phase.
///
/// # Safety
/// Outputs must each be writable for their type for the full call. They must be
/// non-null, naturally aligned, and pairwise nonoverlapping. No pointer is kept.
// SAFETY: unique reserved export; output storage is checked before runtime mutation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ou_lifecycle_next(
    runtime: i64,
    mode: i32,
    token: *mut i64,
    component: *mut i64,
    event: *mut i32,
) -> i32 {
    if !(0..=1).contains(&mode) {
        return INVALID_KIND;
    }
    let ranges = match (
        output_range(token),
        output_range(component),
        output_range(event),
    ) {
        (Ok(token), Ok(component), Ok(event)) => [token, component, event],
        _ => return INVALID_BUFFER,
    };
    for left in 0..ranges.len() {
        for right in left + 1..ranges.len() {
            if ranges[left].0 < ranges[right].1 && ranges[right].0 < ranges[left].1 {
                return INVALID_BUFFER;
            }
        }
    }
    with_runtime(runtime, |runtime| match runtime.lifecycle_next(mode) {
        Ok(Some(delivery)) => {
            // SAFETY: caller promises writable storage; null, alignment, bounds
            // arithmetic, and pairwise overlap were checked before mutation.
            // Only copied fixed-width values escape the store mutex.
            unsafe {
                token.write(delivery.token);
                component.write(delivery.component.to_raw());
                event.write(delivery.event);
            }
            1
        }
        Ok(None) => 0,
        Err(error) => runtime_error(error),
    })
    .unwrap_or_else(|error| error)
}
