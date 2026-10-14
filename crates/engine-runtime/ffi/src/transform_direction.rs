//! Scale-free local/world direction conversion for Transform.

use crate::runtime_store::with_runtime;
use crate::status::{INVALID_BUFFER, INVALID_KIND, object_id, runtime_error};

/// Version of the optional Transform direction-conversion extension.
pub const TRANSFORM_DIRECTION_ABI_VERSION: u32 = 0x0001_0000;

/// Return the direction-conversion ABI version without accessing runtime storage.
// SAFETY: unique symbol within the reserved ABI namespace.
#[unsafe(no_mangle)]
pub extern "C" fn ou_transform_direction_abi_version() -> u32 {
    TRANSFORM_DIRECTION_ABI_VERSION
}

/// Convert a finite direction using only the Transform's world rotation.
///
/// `inverse` must be zero for local-to-world or one for world-to-local.
/// The input and output may overlap because the input is copied before writing.
/// # Safety
/// Input/output must be naturally aligned and provide three initialized
/// readable/writable floats throughout the call. Dangling pointers are
/// undetectable; the regions must not have conflicting concurrent access.
// SAFETY: unique symbol within the reserved ABI namespace.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ou_transform_direction(
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
    if count != 3
        || input.is_null()
        || output.is_null()
        || !input.is_aligned()
        || !output.is_aligned()
    {
        return INVALID_BUFFER;
    }

    with_runtime(runtime, |runtime| {
        let id = match object_id(transform) {
            Ok(id) => id,
            Err(error) => return runtime_error(error),
        };
        let mut direction = [0.0; 3];
        // SAFETY: exact count, pointer alignment, and null checks were completed
        // above; the caller guarantees the input region remains readable.
        unsafe {
            std::ptr::copy_nonoverlapping(input, direction.as_mut_ptr(), direction.len());
        }

        let result = match runtime.transform_direction(id, direction, inverse == 1) {
            Ok(result) => result,
            Err(error) => return runtime_error(error),
        };
        // SAFETY: output is caller-provided writable aligned storage and result
        // is private local memory. Runtime validation completed before writing.
        unsafe {
            std::ptr::copy_nonoverlapping(result.as_ptr(), output, result.len());
        }
        0
    })
    .unwrap_or_else(|error| error)
}
