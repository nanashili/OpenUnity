//! Atomic Transform child detachment transport.

use crate::runtime_store::with_runtime;
use crate::status::{object_id, runtime_error};

/// Version of the optional Transform detach-children extension.
pub const TRANSFORM_DETACH_ABI_VERSION: u32 = 0x0001_0000;

/// Return the detach-children ABI version without accessing runtime storage.
// SAFETY: unique symbol within the reserved ABI namespace.
#[unsafe(no_mangle)]
pub extern "C" fn ou_transform_detach_abi_version() -> u32 {
    TRANSFORM_DETACH_ABI_VERSION
}

/// Detach every direct child to its owning root group while preserving world pose.
// SAFETY: unique symbol within the reserved ABI namespace.
#[unsafe(no_mangle)]
pub extern "C" fn ou_transform_detach_children(runtime: i64, transform: i64) -> i32 {
    with_runtime(runtime, |runtime| {
        object_id(transform)
            .and_then(|id| runtime.transform_detach_children(id))
            .map_or_else(runtime_error, |()| 0)
    })
    .unwrap_or_else(|error| error)
}
