//! Exact-shape transport for copied Rigidbody vector properties.

use crate::runtime_store::with_runtime;
use crate::status::{INVALID_BUFFER, INVALID_KIND, runtime_error};
use engine_object::ObjectId;
use engine_runtime::RuntimeError;

const VECTOR_COUNT: i32 = 3;

fn validate_vector_buffer(pointer: *const f32, count: i32) -> Result<(), i32> {
    if count != VECTOR_COUNT || pointer.is_null() || !pointer.is_aligned() {
        return Err(INVALID_BUFFER);
    }

    pointer
        .addr()
        .checked_add(size_of::<[f32; 3]>())
        .map(|_| ())
        .ok_or(INVALID_BUFFER)
}

fn validate_vector_selector(property: i32) -> Result<(), i32> {
    if (1..=3).contains(&property) {
        Ok(())
    } else {
        Err(INVALID_KIND)
    }
}

/// Read position, linear velocity, or angular velocity into a caller buffer.
/// Validation precedes runtime lookup and a failed read leaves every byte intact.
// SAFETY: caller provides three initialized writable floats for the duration.
///
/// # Safety
/// `output` must point to three writable `f32` values for the duration of this call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ou_rigidbody_vector_read(
    runtime: i64,
    body: i64,
    property: i32,
    output: *mut f32,
    count: i32,
) -> i32 {
    if let Err(error) = validate_vector_selector(property) {
        return error;
    }
    if let Err(error) = validate_vector_buffer(output.cast_const(), count) {
        return error;
    }

    let value = match with_runtime(runtime, |runtime| {
        let body = ObjectId::from_raw(body).ok_or(RuntimeError::InvalidObject)?;
        let state = runtime.rigidbody_state(body)?;
        Ok(match property {
            1 => state.position(),
            2 => state.linear_velocity(),
            3 => state.configuration().angular_velocity(),
            _ => unreachable!("vector selector validated before runtime lookup"),
        })
    }) {
        Ok(Ok(value)) => value,
        Ok(Err(error)) => return runtime_error(error),
        Err(error) => return error,
    };

    // SAFETY: the validated output is live and nonoverlapping by the ABI precondition.
    unsafe { std::ptr::copy_nonoverlapping(value.as_ptr(), output, VECTOR_COUNT as usize) };
    0
}

/// Write a complete position, linear velocity, or angular velocity value.
/// The caller buffer is copied before domain validation or runtime mutation.
// SAFETY: caller provides three initialized readable floats for the duration.
///
/// # Safety
/// `input` must point to three initialized readable `f32` values for the duration
/// of this call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ou_rigidbody_vector_write(
    runtime: i64,
    body: i64,
    property: i32,
    input: *const f32,
    count: i32,
) -> i32 {
    if let Err(error) = validate_vector_selector(property) {
        return error;
    }
    if let Err(error) = validate_vector_buffer(input, count) {
        return error;
    }

    let mut value = [0.0; 3];
    // SAFETY: the validated input is initialized and live by the ABI precondition.
    unsafe { std::ptr::copy_nonoverlapping(input, value.as_mut_ptr(), VECTOR_COUNT as usize) };

    with_runtime(runtime, |runtime| {
        let body = ObjectId::from_raw(body).ok_or(RuntimeError::InvalidObject);
        let body = match body {
            Ok(body) => body,
            Err(error) => return runtime_error(error),
        };
        let result = match property {
            1 => runtime.rigidbody_set_position(body, value),
            2 => runtime.rigidbody_set_linear_velocity(body, value),
            3 => runtime.rigidbody_set_angular_velocity(body, value),
            _ => unreachable!("vector selector validated before runtime lookup"),
        };
        result.map_or_else(runtime_error, |_| 0)
    })
    .unwrap_or_else(|error| error)
}
