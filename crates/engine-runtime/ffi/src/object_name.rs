//! Bounded owned UTF-8 name transport; borrowed storage never escapes a call.

use crate::runtime_store::with_runtime;
use crate::status::{INSUFFICIENT_CAPACITY, INVALID_BUFFER, INVALID_OBJECT, INVALID_UTF8};
use engine_object::ObjectId;
use engine_runtime::MAX_OBJECT_NAME_BYTES;

pub(crate) fn buffer_len(pointer: *const u8, count: i32) -> Result<usize, i32> {
    let count = usize::try_from(count).map_err(|_| INVALID_BUFFER)?;
    if count > MAX_OBJECT_NAME_BYTES || (count != 0 && pointer.is_null()) {
        return Err(INVALID_BUFFER);
    }
    Ok(count)
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
