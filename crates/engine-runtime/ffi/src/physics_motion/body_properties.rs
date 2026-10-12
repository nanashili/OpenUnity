//! Scalar, flag, interpolation, and wake-up transport for one Rigidbody.

use crate::runtime_store::with_runtime;
use crate::status::{INVALID_BUFFER, INVALID_KIND, runtime_error};
use engine_object::ObjectId;
use engine_physics_api::BodyInterpolation;
use engine_runtime::RuntimeError;

fn scalar_output(output: *mut f32) -> Result<(), i32> {
    if output.is_null() || !output.is_aligned() {
        return Err(INVALID_BUFFER);
    }
    output
        .addr()
        .checked_add(size_of::<f32>())
        .map(|_| ())
        .ok_or(INVALID_BUFFER)
}

fn scalar_selector(property: i32) -> Result<(), i32> {
    if (1..=4).contains(&property) {
        Ok(())
    } else {
        Err(INVALID_KIND)
    }
}

fn flag_selector(property: i32) -> Result<(), i32> {
    if (1..=4).contains(&property) {
        Ok(())
    } else {
        Err(INVALID_KIND)
    }
}

/// Read mass, linear damping, angular damping, or sleep threshold.
// SAFETY: caller provides one naturally aligned writable float for the duration.
///
/// # Safety
/// `output` must point to one writable `f32` for the duration of this call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ou_rigidbody_scalar_read(
    runtime: i64,
    body: i64,
    property: i32,
    output: *mut f32,
) -> i32 {
    if let Err(error) = scalar_selector(property) {
        return error;
    }
    if let Err(error) = scalar_output(output) {
        return error;
    }

    let value = match with_runtime(runtime, |runtime| {
        let body = ObjectId::from_raw(body).ok_or(RuntimeError::InvalidObject)?;
        let state = runtime.rigidbody_state(body)?;
        let profile = state.configuration();
        Ok(match property {
            1 => profile.mass(),
            2 => profile.linear_damping(),
            3 => profile.angular_damping(),
            4 => profile.sleep_threshold(),
            _ => unreachable!("scalar selector validated before runtime lookup"),
        })
    }) {
        Ok(Ok(value)) => value,
        Ok(Err(error)) => return runtime_error(error),
        Err(error) => return error,
    };

    // SAFETY: output passed shape, alignment, and address-range checks.
    unsafe { std::ptr::write(output, value) };
    0
}

/// Replace one supported scalar property after copying and validating it privately.
#[unsafe(no_mangle)]
pub extern "C" fn ou_rigidbody_scalar_write(
    runtime: i64,
    body: i64,
    property: i32,
    value: f32,
) -> i32 {
    if let Err(error) = scalar_selector(property) {
        return error;
    }
    with_runtime(runtime, |runtime| {
        let body = match ObjectId::from_raw(body) {
            Some(body) => body,
            None => return runtime_error(RuntimeError::InvalidObject),
        };
        let result = match property {
            1 => runtime.rigidbody_set_mass(body, value),
            2 => runtime.rigidbody_set_linear_damping(body, value),
            3 => runtime.rigidbody_set_angular_damping(body, value),
            4 => runtime.rigidbody_set_sleep_threshold(body, value),
            _ => unreachable!("scalar selector validated before runtime lookup"),
        };
        result.map_or_else(runtime_error, |_| 0)
    })
    .unwrap_or_else(|error| error)
}

/// Read useGravity, isKinematic, detectCollisions, or sleeping as 0/1.
// SAFETY: caller provides one naturally aligned writable i32 for the duration.
///
/// # Safety
/// `output` must point to one writable `i32` for the duration of this call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ou_rigidbody_flag_read(
    runtime: i64,
    body: i64,
    property: i32,
    output: *mut i32,
) -> i32 {
    if let Err(error) = flag_selector(property) {
        return error;
    }
    if output.is_null() || !output.is_aligned() {
        return INVALID_BUFFER;
    }
    if output.addr().checked_add(size_of::<i32>()).is_none() {
        return INVALID_BUFFER;
    }

    let value = match with_runtime(runtime, |runtime| {
        let body = ObjectId::from_raw(body).ok_or(RuntimeError::InvalidObject)?;
        let state = runtime.rigidbody_state(body)?;
        let profile = state.configuration();
        Ok(match property {
            1 => profile.use_gravity(),
            2 => profile.is_kinematic(),
            3 => profile.detect_collisions(),
            4 => state.is_sleeping(),
            _ => unreachable!("flag selector validated before runtime lookup"),
        })
    }) {
        Ok(Ok(value)) => value,
        Ok(Err(error)) => return runtime_error(error),
        Err(error) => return error,
    };

    // SAFETY: output passed pointer validation and the ABI writable-memory precondition.
    unsafe { std::ptr::write(output, i32::from(value)) };
    0
}

/// Replace useGravity, isKinematic, or detectCollisions. Sleeping is read-only.
#[unsafe(no_mangle)]
pub extern "C" fn ou_rigidbody_flag_write(
    runtime: i64,
    body: i64,
    property: i32,
    value: i32,
) -> i32 {
    if let Err(error) = flag_selector(property) {
        return error;
    }
    if property == 4 || !(0..=1).contains(&value) {
        return INVALID_KIND;
    }
    with_runtime(runtime, |runtime| {
        let body = match ObjectId::from_raw(body) {
            Some(body) => body,
            None => return runtime_error(RuntimeError::InvalidObject),
        };
        let result = match property {
            1 => runtime.rigidbody_set_use_gravity(body, value == 1),
            2 => runtime.rigidbody_set_is_kinematic(body, value == 1),
            3 => runtime.rigidbody_set_detect_collisions(body, value == 1),
            4 => unreachable!("sleeping flag is read-only"),
            _ => unreachable!("flag selector validated before runtime lookup"),
        };
        result.map_or_else(runtime_error, |_| 0)
    })
    .unwrap_or_else(|error| error)
}

/// Read the sole supported interpolation value, encoded as zero.
// SAFETY: caller provides one naturally aligned writable i32 for the duration.
///
/// # Safety
/// `output` must point to one writable `i32` for the duration of this call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ou_rigidbody_interpolation_read(
    runtime: i64,
    body: i64,
    output: *mut i32,
) -> i32 {
    if output.is_null() || !output.is_aligned() {
        return INVALID_BUFFER;
    }
    if output.addr().checked_add(size_of::<i32>()).is_none() {
        return INVALID_BUFFER;
    }
    let value = match with_runtime(runtime, |runtime| {
        let body = ObjectId::from_raw(body).ok_or(RuntimeError::InvalidObject)?;
        Ok(runtime
            .rigidbody_state(body)?
            .configuration()
            .interpolation())
    }) {
        Ok(Ok(BodyInterpolation::None)) => 0,
        Ok(Err(error)) => return runtime_error(error),
        Err(error) => return error,
    };
    // SAFETY: output passed pointer validation and the ABI writable-memory precondition.
    unsafe { std::ptr::write(output, value) };
    0
}

/// Set interpolation. Only native encoding zero is admitted.
#[unsafe(no_mangle)]
pub extern "C" fn ou_rigidbody_interpolation_write(runtime: i64, body: i64, value: i32) -> i32 {
    if value != 0 {
        return INVALID_KIND;
    }
    with_runtime(runtime, |runtime| {
        let body = match ObjectId::from_raw(body) {
            Some(body) => body,
            None => return runtime_error(RuntimeError::InvalidObject),
        };
        runtime
            .rigidbody_set_interpolation(body, BodyInterpolation::None)
            .map_or_else(runtime_error, |_| 0)
    })
    .unwrap_or_else(|error| error)
}

/// Wake a published body without exposing a borrowed backend reference.
#[unsafe(no_mangle)]
pub extern "C" fn ou_rigidbody_wake_up(runtime: i64, body: i64) -> i32 {
    with_runtime(runtime, |runtime| {
        let body = match ObjectId::from_raw(body) {
            Some(body) => body,
            None => return runtime_error(RuntimeError::InvalidObject),
        };
        runtime
            .rigidbody_wake_up(body)
            .map_or_else(runtime_error, |_| 0)
    })
    .unwrap_or_else(|error| error)
}
