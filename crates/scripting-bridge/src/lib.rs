//! OpenUnity C ABI v1 with an independently versioned owned UTF-8 name extension.
//! No Unity assemblies or runtime are loaded.
//!
//! All exports use the reserved `ou_` prefix. Name buffers are copied during the
//! call under the runtime-store mutex; no borrowed storage pointer escapes.
//! Unexpected internal panics and allocation failure terminate the process;
//! ordinary invalid input, identity exhaustion and poisoned storage return errors.

use openunity_object_model::{ObjectId, Registry};
use openunity_runtime::{MAX_OBJECT_NAME_BYTES, Runtime, RuntimeError};
use std::sync::{Mutex, OnceLock};
mod lifecycle;
mod scene;
mod timing;
pub use lifecycle::*;
pub use scene::*;
pub use timing::*;

/// ABI major 1, minor 0.
pub const ABI_VERSION: u32 = 0x0001_0000;
/// Independent object name ABI major 1, minor 0.
pub const OBJECT_NAME_ABI_VERSION: u32 = 0x0001_0000;
/// Invalid or destroyed runtime context.
pub const INVALID_RUNTIME: i32 = -1;
/// Invalid, stale, or foreign object identity.
pub const INVALID_OBJECT: i32 = -2;
/// Storage poisoned by an internal failure.
pub const STORE_UNAVAILABLE: i32 = -3;
/// Process identity space exhausted without wrapping.
pub const IDENTITY_EXHAUSTED: i32 = -4;
/// Invalid name buffer pointer or byte count.
pub const INVALID_BUFFER: i32 = -5;
/// Name input is not strict UTF-8.
pub const INVALID_UTF8: i32 = -6;
/// Read capacity is smaller than the complete current name.
pub const INSUFFICIENT_CAPACITY: i32 = -7;

static RUNTIMES: OnceLock<Mutex<Registry<Runtime>>> = OnceLock::new();

fn store() -> &'static Mutex<Registry<Runtime>> {
    RUNTIMES.get_or_init(|| Mutex::new(Registry::default()))
}

fn with_runtime<T>(raw: i64, f: impl FnOnce(&mut Runtime) -> T) -> Result<T, i32> {
    let id = ObjectId::from_raw(raw).ok_or(INVALID_RUNTIME)?;
    let mut runtimes = store().lock().map_err(|_| STORE_UNAVAILABLE)?;
    let runtime = runtimes.get_mut(id).ok_or(INVALID_RUNTIME)?;
    if runtime.scene_staging() {
        return Err(STAGING_BUSY);
    }
    Ok(f(runtime))
}

fn buffer_len(pointer: *const u8, count: i32) -> Result<usize, i32> {
    let count = usize::try_from(count).map_err(|_| INVALID_BUFFER)?;
    if count > MAX_OBJECT_NAME_BYTES || (count != 0 && pointer.is_null()) {
        return Err(INVALID_BUFFER);
    }
    Ok(count)
}

/// Return the integer-only ABI version without accessing the runtime store.
// SAFETY: the ou_ namespace is reserved for this ABI and has one definition per symbol.
#[unsafe(no_mangle)]
pub extern "C" fn ou_abi_version() -> u32 {
    ABI_VERSION
}

/// Return the independent name-extension version without accessing storage.
/// Clients must check this version and [`ou_abi_version`] before name calls.
// SAFETY: unique symbol in the reserved ABI namespace; fixed-width integer result.
#[unsafe(no_mangle)]
pub extern "C" fn ou_object_name_abi_version() -> u32 {
    OBJECT_NAME_ABI_VERSION
}

/// Create a context, returning a positive identity or a negative error code.
// SAFETY: unique symbol in the reserved ABI namespace; fixed-width integer interface.
#[unsafe(no_mangle)]
pub extern "C" fn ou_runtime_create() -> i64 {
    match store().lock() {
        Ok(mut runtimes) => runtimes
            .insert(Runtime::default())
            .map(ObjectId::to_raw)
            .unwrap_or(i64::from(IDENTITY_EXHAUSTED)),
        Err(_) => i64::from(STORE_UNAVAILABLE),
    }
}

/// Destroy the context and its objects, returning zero or a negative error code.
// SAFETY: unique symbol in the reserved ABI namespace; fixed-width integer interface.
#[unsafe(no_mangle)]
pub extern "C" fn ou_runtime_destroy(runtime: i64) -> i32 {
    let Some(id) = ObjectId::from_raw(runtime) else {
        return INVALID_RUNTIME;
    };
    match store().lock() {
        Ok(mut runtimes) => {
            if runtimes.get(id).is_some_and(Runtime::scene_staging) {
                return STAGING_BUSY;
            }
            if runtimes.remove(id).is_some() {
                0
            } else {
                INVALID_RUNTIME
            }
        }
        Err(_) => STORE_UNAVAILABLE,
    }
}

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

/// Return the current name's UTF-8 byte count or a negative error code.
/// A later read can return [`INSUFFICIENT_CAPACITY`] after concurrent mutation.
// SAFETY: unique symbol in the reserved ABI namespace; fixed-width integer interface.
#[unsafe(no_mangle)]
pub extern "C" fn ou_object_name_len(runtime: i64, object: i64) -> i32 {
    with_runtime(runtime, |runtime| {
        ObjectId::from_raw(object)
            .and_then(|id| runtime.object_name(id))
            .map_or(INVALID_OBJECT, |name| name.len() as i32)
    })
    .unwrap_or_else(|error| error)
}

/// Copy the complete UTF-8 name without a terminator, returning its byte count.
/// Invalid buffers/counts precede runtime/store errors, then object errors, then
/// insufficient capacity. Failure leaves output untouched; extra bytes stay unchanged.
///
/// # Safety
/// For nonzero capacity, output must reference a writable region of at least
/// capacity bytes, valid for the entire call, with no conflicting concurrent
/// access. Non-null dangling pointers cannot be detected. Null is permitted
/// only for zero capacity, which succeeds only when the name is empty.
// SAFETY: unique symbol in the reserved ABI namespace; pointer obligations above.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ou_object_name_read(
    runtime: i64,
    object: i64,
    output: *mut u8,
    capacity: i32,
) -> i32 {
    let capacity = match buffer_len(output.cast_const(), capacity) {
        Ok(capacity) => capacity,
        Err(error) => return error,
    };
    with_runtime(runtime, |runtime| {
        let Some(name) = ObjectId::from_raw(object).and_then(|id| runtime.object_name(id)) else {
            return INVALID_OBJECT;
        };
        if capacity < name.len() {
            return INSUFFICIENT_CAPACITY;
        }
        if !name.is_empty() {
            // SAFETY: the caller provides capacity writable bytes, capacity covers
            // the whole name, and private registry storage cannot alias output.
            // The mutex keeps the source valid through this complete copy.
            unsafe { std::ptr::copy_nonoverlapping(name.as_ptr(), output, name.len()) };
        }
        name.len() as i32
    })
    .unwrap_or_else(|error| error)
}

/// Replace the complete name with an owned strict UTF-8 copy, returning zero.
/// Invalid buffers/counts precede runtime/store errors, then object errors, then
/// invalid UTF-8. Every failure preserves the previous name. Embedded zeroes
/// are contents; a null zero-length input sets an empty name.
///
/// # Safety
/// For nonzero length, input must reference at least length initialized readable
/// bytes, valid for the entire call, with no conflicting concurrent access.
/// Non-null dangling pointers cannot be detected. Null is permitted only for
/// zero length; no slice is constructed from a null pointer.
// SAFETY: unique symbol in the reserved ABI namespace; pointer obligations above.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ou_object_name_set(
    runtime: i64,
    object: i64,
    input: *const u8,
    length: i32,
) -> i32 {
    let length = match buffer_len(input, length) {
        Ok(length) => length,
        Err(error) => return error,
    };
    with_runtime(runtime, |runtime| {
        let Some(id) = ObjectId::from_raw(object).filter(|id| runtime.is_alive(*id)) else {
            return INVALID_OBJECT;
        };
        let bytes = if length == 0 {
            &[]
        } else {
            // SAFETY: nonzero validated length and the caller's readable-region
            // obligation permit this bounded slice for the duration of the call.
            unsafe { std::slice::from_raw_parts(input, length) }
        };
        let Ok(name) = std::str::from_utf8(bytes) else {
            return INVALID_UTF8;
        };
        if runtime.set_object_name(id, name) {
            0
        } else {
            INVALID_OBJECT
        }
    })
    .unwrap_or_else(|error| error)
}

/// Independent Transform extension major 1, minor 0.
pub const TRANSFORM_ABI_VERSION: u32 = 0x0001_0000;
/// Invalid typed kind, property, index, or flag.
pub const INVALID_KIND: i32 = -8;
/// Invalid numeric input or result.
pub const INVALID_NUMERIC: i32 = -9;
/// Reparenting would create an ancestor cycle.
pub const HIERARCHY_CYCLE: i32 = -10;
/// Singular inverse or unsupported world-preserving reparent.
pub const UNSUPPORTED_TRANSFORM: i32 = -11;
/// Native count exceeds the signed ABI representation.
pub const COUNT_OVERFLOW: i32 = -12;

fn runtime_error(error: RuntimeError) -> i32 {
    match error {
        RuntimeError::InvalidObject => INVALID_OBJECT,
        RuntimeError::InvalidKind => INVALID_KIND,
        RuntimeError::Numeric => INVALID_NUMERIC,
        RuntimeError::Cycle => HIERARCHY_CYCLE,
        RuntimeError::Unsupported => UNSUPPORTED_TRANSFORM,
        RuntimeError::CountOverflow => COUNT_OVERFLOW,
        RuntimeError::Lifecycle => INVALID_LIFECYCLE,
        RuntimeError::DispatchFaulted => GAMEPLAY_FAULTED,
        RuntimeError::IdentityExhausted => IDENTITY_EXHAUSTED,
        RuntimeError::InvalidScene => INVALID_SCENE,
        RuntimeError::InvalidScenePlan => INVALID_SCENE_PLAN,
        RuntimeError::StagingBusy => STAGING_BUSY,
    }
}
fn object_id(raw: i64) -> Result<ObjectId, RuntimeError> {
    ObjectId::from_raw(raw).ok_or(RuntimeError::InvalidObject)
}
fn property_count(property: i32) -> Result<usize, i32> {
    match property {
        1 | 3 | 4 => Ok(3),
        2 | 5 => Ok(4),
        _ => Err(INVALID_KIND),
    }
}
fn float_buffer(pointer: *const f32, count: i32, expected: usize) -> Result<(), i32> {
    if usize::try_from(count).ok() != Some(expected) || pointer.is_null() || !pointer.is_aligned() {
        Err(INVALID_BUFFER)
    } else {
        Ok(())
    }
}
fn transform_id(runtime: &Runtime, raw: i64) -> Result<ObjectId, RuntimeError> {
    let id = object_id(raw)?;
    if runtime.object_kind(id)? == 3 {
        Ok(id)
    } else {
        Err(RuntimeError::InvalidKind)
    }
}

/// Return the independent typed-extension version without storage access.
// SAFETY: unique symbol within the reserved ABI namespace.
#[unsafe(no_mangle)]
pub extern "C" fn ou_transform_abi_version() -> u32 {
    TRANSFORM_ABI_VERSION
}

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

/// Copy an approved pose property to the exact-size output buffer on success.
/// Property, exact count, and pointer validation precede runtime lookup.
/// # Safety
/// Output must be naturally aligned and writable for count floats throughout the
/// call, without conflicting concurrent access. Dangling pointers are undetectable.
// SAFETY: unique symbol within the reserved ABI namespace.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ou_transform_read(
    runtime: i64,
    transform: i64,
    property: i32,
    output: *mut f32,
    count: i32,
) -> i32 {
    let expected = match property_count(property) {
        Ok(count) => count,
        Err(error) => return error,
    };
    if let Err(error) = float_buffer(output.cast_const(), count, expected) {
        return error;
    }
    with_runtime(runtime, |runtime| {
        let values = match object_id(transform).and_then(|id| runtime.transform_read(id, property))
        {
            Ok(values) => values,
            Err(error) => return runtime_error(error),
        };
        // SAFETY: validation and caller guarantee count writable aligned floats;
        // values is private owned memory of exactly the prescribed count and
        // cannot alias the caller's buffer. All validation finished before copy.
        unsafe {
            std::ptr::copy_nonoverlapping(values.as_ptr(), output, expected);
        }
        0
    })
    .unwrap_or_else(|error| error)
}

/// Atomically assign an approved pose property from an exact-size input buffer.
/// # Safety
/// Input must be naturally aligned and initialized/readable for count floats
/// throughout the call, without conflicting concurrent access.
// SAFETY: unique symbol within the reserved ABI namespace.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ou_transform_write(
    runtime: i64,
    transform: i64,
    property: i32,
    input: *const f32,
    count: i32,
) -> i32 {
    let expected = match property_count(property) {
        Ok(count) => count,
        Err(error) => return error,
    };
    if let Err(error) = float_buffer(input, count, expected) {
        return error;
    }
    with_runtime(runtime, |runtime| {
        let id = match transform_id(runtime, transform) {
            Ok(id) => id,
            Err(error) => return runtime_error(error),
        };
        let mut values = [0.0; 4];
        // SAFETY: exact validated bounded count and caller's readable region;
        // local owned array cannot overlap the caller's input. No pointer escapes.
        unsafe {
            std::ptr::copy_nonoverlapping(input, values.as_mut_ptr(), expected);
        }
        runtime
            .transform_write(id, property, &values[..expected])
            .map_or_else(runtime_error, |()| 0)
    })
    .unwrap_or_else(|error| error)
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

/// Convert an affine point, permitting overlapping input and output regions.
/// Scalar flag, exact count, and both pointers validate before runtime access.
/// # Safety
/// Input/output must be naturally aligned and provide three initialized readable
/// input/writable output floats throughout the call. The regions may overlap but
/// must have no conflicting concurrent access. Dangling pointers are undetectable.
// SAFETY: unique symbol within the reserved ABI namespace.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ou_transform_point(
    runtime: i64,
    transform: i64,
    input: *const f32,
    output: *mut f32,
    count: i32,
    inverse: i32,
) -> i32 {
    if !matches!(inverse, 0 | 1) {
        return INVALID_KIND;
    }
    if let Err(error) =
        float_buffer(input, count, 3).and_then(|()| float_buffer(output.cast_const(), count, 3))
    {
        return error;
    }
    with_runtime(runtime, |runtime| {
        let id = match transform_id(runtime, transform) {
            Ok(id) => id,
            Err(error) => return runtime_error(error),
        };
        let mut point = [0.0; 3];
        // SAFETY: validated count/alignment and caller's readable region; copying
        // all inputs to private local storage allows arbitrary input/output overlap.
        unsafe {
            std::ptr::copy_nonoverlapping(input, point.as_mut_ptr(), 3);
        }
        let result = match runtime.transform_point(id, point, inverse == 1) {
            Ok(result) => result,
            Err(error) => return runtime_error(error),
        };
        // SAFETY: output is caller-provided writable aligned storage; result is
        // private local memory. All validation and arithmetic succeeded first.
        unsafe {
            std::ptr::copy_nonoverlapping(result.as_ptr(), output, 3);
        }
        0
    })
    .unwrap_or_else(|error| error)
}
