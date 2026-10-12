//! Additive native timing ABI; caller storage never becomes runtime-owned.

use crate::runtime_store::with_runtime;
use crate::status::{INVALID_BUFFER, INVALID_KIND, input_status, runtime_error};
use engine_runtime::RuntimeError;
use engine_runtime::{validate_time_configuration, validate_time_property};

/// Independent timing capability major 1, minor 0.
pub const TIME_ABI_VERSION: u32 = 0x0001_0000;

fn status(result: Result<(), RuntimeError>) -> i32 {
    result.map_or_else(runtime_error, |()| 0)
}

fn output_range<T>(pointer: *mut T, count: usize) -> Result<(usize, usize), i32> {
    if pointer.is_null() || !pointer.is_aligned() {
        return Err(INVALID_BUFFER);
    }
    let bytes = size_of::<T>().checked_mul(count).ok_or(INVALID_BUFFER)?;
    let start = pointer.addr();
    let end = start.checked_add(bytes).ok_or(INVALID_BUFFER)?;
    Ok((start, end))
}

fn disjoint(ranges: &[(usize, usize)]) -> Result<(), i32> {
    for (index, left) in ranges.iter().enumerate() {
        for right in &ranges[index + 1..] {
            if left.0 < right.1 && right.0 < left.1 {
                return Err(INVALID_BUFFER);
            }
        }
    }
    Ok(())
}

/// Return the timing capability without touching runtime storage.
// SAFETY: unique reserved symbol with a fixed-width scalar result.
#[unsafe(no_mangle)]
pub extern "C" fn ou_time_abi_version() -> u32 {
    TIME_ABI_VERSION
}

/// Enable timing once on an unused, healthy native frame context.
// SAFETY: unique reserved symbol; no pointers cross this interface.
#[unsafe(no_mangle)]
pub extern "C" fn ou_time_configure(runtime: i64, source: i32, step: f64) -> i32 {
    if let Err(error) = validate_time_configuration(source, step) {
        return runtime_error(error);
    }
    with_runtime(runtime, |runtime| {
        status(runtime.time_configure(source, step))
    })
    .unwrap_or_else(|error| error)
}

/// Set one finite, bounded native timing property.
// SAFETY: unique reserved symbol; no pointers cross this interface.
#[unsafe(no_mangle)]
pub extern "C" fn ou_time_set(runtime: i64, property: i32, value: f32) -> i32 {
    if let Err(error) = validate_time_property(property, value) {
        return runtime_error(error);
    }
    with_runtime(runtime, |runtime| status(runtime.time_set(property, value)))
        .unwrap_or_else(|error| error)
}

fn snapshot_ranges(
    values: *mut f64,
    count: i32,
    frame: *mut i32,
    fixed_context: *mut i32,
) -> Result<(), i32> {
    if count != 6 {
        return Err(INVALID_BUFFER);
    }
    let ranges = [
        output_range(values, 6)?,
        output_range(frame, 1)?,
        output_range(fixed_context, 1)?,
    ];
    disjoint(&ranges)
}

/// Copy one complete value-only timing snapshot; failures leave outputs unchanged.
/// # Safety
/// All output ranges must be live, writable and correctly sized for the entire
/// call. The bridge checks shape and address arithmetic, not memory mapping.
// SAFETY: unique reserved symbol; bounded output shape is checked before lookup.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ou_time_snapshot(
    runtime: i64,
    values: *mut f64,
    count: i32,
    frame: *mut i32,
    fixed_context: *mut i32,
) -> i32 {
    if let Err(error) = snapshot_ranges(values, count, frame, fixed_context) {
        return error;
    }
    let snapshot = match with_runtime(runtime, |runtime| runtime.time_snapshot()) {
        Ok(Ok(snapshot)) => snapshot,
        Ok(Err(error)) => return runtime_error(error),
        Err(error) => return error,
    };
    // SAFETY: caller promises live writable storage; shape and disjointness were
    // checked. Only copied scalar values escape the released runtime-store lock.
    unsafe {
        values.copy_from_nonoverlapping(snapshot.values.as_ptr(), 6);
        frame.write(snapshot.frame);
        fixed_context.write(i32::from(snapshot.fixed_context));
    }
    0
}

/// Begin one timed frame without interpreting any old host tick integer.
// SAFETY: unique reserved symbol with a fixed-width scalar interface.
#[unsafe(no_mangle)]
pub extern "C" fn ou_time_begin_frame(runtime: i64) -> i32 {
    with_runtime(runtime, |runtime| input_status(runtime.time_begin_frame()))
        .unwrap_or_else(|error| error)
}

/// Begin one due fixed iteration, or return zero when fixed work is complete.
// SAFETY: unique reserved symbol with a fixed-width scalar interface.
#[unsafe(no_mangle)]
pub extern "C" fn ou_time_begin_fixed(runtime: i64) -> i32 {
    with_runtime(runtime, |runtime| {
        runtime
            .time_begin_fixed()
            .map_or_else(runtime_error, i32::from)
    })
    .unwrap_or_else(|error| error)
}

/// Copy one genuine native delivery, including additive event 8 for FixedUpdate.
/// On zero or error every output remains unchanged.
/// # Safety
/// Each pointer must name live, correctly sized writable storage for the entire
/// call. The bridge validates nonnull/alignment/overflow/nonoverlap only.
// SAFETY: unique reserved symbol; output shape is checked before runtime mutation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ou_time_next(
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
        output_range(token, 1),
        output_range(component, 1),
        output_range(event, 1),
    ) {
        (Ok(token), Ok(component), Ok(event)) => [token, component, event],
        _ => return INVALID_BUFFER,
    };
    if let Err(error) = disjoint(&ranges) {
        return error;
    }
    let delivery = match with_runtime(runtime, |runtime| runtime.time_next(mode)) {
        Ok(Ok(Some(delivery))) => delivery,
        Ok(Ok(None)) => return 0,
        Ok(Err(error)) => return runtime_error(error),
        Err(error) => return error,
    };
    // SAFETY: caller promises live writable storage; bounded output shape was
    // checked before selection. No borrowed runtime state crosses this boundary.
    unsafe {
        token.write(delivery.token);
        component.write(delivery.component.to_raw());
        event.write(delivery.event);
    }
    1
}
