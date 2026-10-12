//! Mode, Transform synchronization, and explicit manual-step transport.

use crate::runtime_store::with_runtime;
use crate::status::{INVALID_KIND, runtime_error};

/// Read native mode: zero is Fixed and one is Script.
#[unsafe(no_mangle)]
pub extern "C" fn ou_physics_mode_read(runtime: i64) -> i32 {
    match with_runtime(runtime, |runtime| runtime.physics_mode_read()) {
        Ok(mode @ (0 | 1)) => mode,
        Ok(_) => INVALID_KIND,
        Err(error) => error,
    }
}

/// Set native mode after validating the independent ABI encoding.
#[unsafe(no_mangle)]
pub extern "C" fn ou_physics_mode_write(runtime: i64, mode: i32) -> i32 {
    if !(0..=1).contains(&mode) {
        return INVALID_KIND;
    }
    with_runtime(runtime, |runtime| {
        runtime
            .physics_mode_write(mode)
            .map_or_else(runtime_error, |_| 0)
    })
    .unwrap_or_else(|error| error)
}

/// Synchronize authored Transform state into the native body, if attached.
#[unsafe(no_mangle)]
pub extern "C" fn ou_physics_sync_transforms(runtime: i64) -> i32 {
    with_runtime(runtime, |runtime| {
        runtime
            .physics_sync_transforms()
            .map_or_else(runtime_error, |_| 0)
    })
    .unwrap_or_else(|error| error)
}

/// Run one positive Script-mode manual step.
#[unsafe(no_mangle)]
pub extern "C" fn ou_physics_simulate(runtime: i64, duration: f32) -> i32 {
    with_runtime(runtime, |runtime| {
        runtime
            .physics_simulate(duration)
            .map_or_else(runtime_error, |_| 0)
    })
    .unwrap_or_else(|error| error)
}
