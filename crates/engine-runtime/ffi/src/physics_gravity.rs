//! Versioned transport for native-owned, finite 3D gravity configuration.

use crate::runtime_store::with_runtime;
use crate::status::{INVALID_BUFFER, runtime_error};

/// Gravity configuration ABI major 1, minor 0, independent of the base ABI.
pub const PHYSICS_GRAVITY_ABI_VERSION: u32 = 0x0001_0000;

fn gravity_buffer(pointer: *const f32, count: i32) -> Result<(), i32> {
    if count != 3 || pointer.is_null() || !pointer.is_aligned() {
        return Err(INVALID_BUFFER);
    }

    pointer
        .addr()
        .checked_add(size_of::<[f32; 3]>())
        .map(|_| ())
        .ok_or(INVALID_BUFFER)
}

/// Query the additive gravity capability without accessing a runtime context.
// SAFETY: unique symbol in the reserved ABI namespace; fixed-width result.
#[unsafe(no_mangle)]
pub extern "C" fn ou_physics_gravity_abi_version() -> u32 {
    PHYSICS_GRAVITY_ABI_VERSION
}

/// Copy the complete configured gravity value only after successful lookup.
/// Exact count, pointer alignment and address range validate before runtime lookup.
/// Failed reads preserve every output byte.
/// # Safety
/// Output must provide three naturally aligned writable floats, live throughout
/// the call without conflicting concurrent access. Pointer checks cannot detect
/// dangling or unmapped storage. No caller pointer or runtime borrow escapes.
// SAFETY: unique symbol in the reserved ABI namespace.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ou_physics_gravity_read(
    runtime: i64,
    output: *mut f32,
    count: i32,
) -> i32 {
    if let Err(error) = gravity_buffer(output.cast_const(), count) {
        return error;
    }

    let gravity = match with_runtime(runtime, |runtime| runtime.physics_gravity()) {
        Ok(gravity) => gravity,
        Err(error) => return error,
    };

    // SAFETY: buffer validation and the caller guarantee three writable floats;
    // the complete owned value cannot overlap caller storage. Lookup succeeded
    // and no runtime borrow remains during the copy.
    unsafe {
        std::ptr::copy_nonoverlapping(gravity.as_ptr(), output, 3);
    }
    0
}

/// Replace the complete configured gravity value from a private input copy.
/// Buffer validation precedes runtime lookup; nonfinite input returns -9 without
/// changing any native component. Calls serialize through the runtime store.
/// # Safety
/// Input must provide three naturally aligned initialized readable floats, live
/// throughout the call without conflicting concurrent access. Pointer checks
/// cannot detect dangling or unmapped storage. No caller pointer escapes.
// SAFETY: unique symbol in the reserved ABI namespace.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ou_physics_gravity_write(
    runtime: i64,
    input: *const f32,
    count: i32,
) -> i32 {
    if let Err(error) = gravity_buffer(input, count) {
        return error;
    }

    with_runtime(runtime, |runtime| {
        let mut gravity = [0.0; 3];
        // SAFETY: buffer validation and caller guarantee three readable floats;
        // the private local array cannot overlap caller input. The whole input
        // is copied before domain validation or any configuration mutation.
        unsafe {
            std::ptr::copy_nonoverlapping(input, gravity.as_mut_ptr(), 3);
        }

        runtime
            .set_physics_gravity(gravity)
            .map_or_else(runtime_error, |()| 0)
    })
    .unwrap_or_else(|error| error)
}
