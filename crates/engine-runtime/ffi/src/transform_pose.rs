//! Exact-size pose buffer transport and overlap-safe affine point conversion.

use crate::runtime_store::with_runtime;
use crate::status::{INVALID_BUFFER, INVALID_KIND, object_id, runtime_error};
use engine_object::ObjectId;
use engine_runtime::{Runtime, RuntimeError};

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
