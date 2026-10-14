//! Lifecycle capability version, boolean flags and operation-status translation.

use crate::status::{INVALID_KIND, runtime_error};
use engine_runtime::RuntimeError;

/// Independent lifecycle extension major 2, minor 0, including Transform callbacks.
pub const LIFECYCLE_ABI_VERSION: u32 = 0x0002_0000;

pub(super) fn status(result: Result<(), RuntimeError>) -> i32 {
    result.map_or_else(runtime_error, |()| 0)
}

pub(super) fn flag(value: i32) -> Result<bool, i32> {
    match value {
        0 => Ok(false),
        1 => Ok(true),
        _ => Err(INVALID_KIND),
    }
}

/// Return the additive lifecycle capability without touching runtime storage.
// SAFETY: unique reserved export with fixed-width integer representation.
#[unsafe(no_mangle)]
pub extern "C" fn ou_lifecycle_abi_version() -> u32 {
    LIFECYCLE_ABI_VERSION
}
