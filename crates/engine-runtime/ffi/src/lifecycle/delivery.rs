//! Validated value-only callback delivery and acknowledgement.

use super::protocol::status;
use crate::runtime_store::with_runtime;
use crate::status::{INVALID_BUFFER, INVALID_KIND, INVALID_LIFECYCLE, runtime_error};

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
