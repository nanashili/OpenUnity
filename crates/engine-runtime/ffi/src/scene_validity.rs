//! Scene validity ABI extension.

use crate::runtime_store::with_runtime_scene_snapshot;
use crate::status::runtime_error;

/// Scene validity ABI version 1.0.
pub const SCENE_VALIDITY_ABI_VERSION: u32 = 0x0001_0000;

/// Return the independent Scene Validity ABI version.
// SAFETY: unique symbol with a fixed-width scalar result.
#[unsafe(no_mangle)]
pub extern "C" fn ou_scene_validity_abi_version() -> u32 {
    SCENE_VALIDITY_ABI_VERSION
}

/// Return whether a scene handle is published and available in this runtime.
// SAFETY: unique symbol with fixed-width integer arguments and result.
#[unsafe(no_mangle)]
pub extern "C" fn ou_scene_is_valid(runtime: i64, scene: i64) -> i32 {
    with_runtime_scene_snapshot(runtime, scene, |runtime, scene| {
        runtime.scene_is_valid(scene)
    })
    .and_then(|result| result.map_err(runtime_error))
    .map_or_else(i32::from, i32::from)
}
