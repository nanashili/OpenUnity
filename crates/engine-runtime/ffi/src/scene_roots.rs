//! Atomic, value-owned scene-root identity snapshots.

use crate::runtime_store::with_runtime;
use crate::status::{
    ALLOCATION_FAILURE, COUNT_OVERFLOW, INSUFFICIENT_CAPACITY, INVALID_BUFFER, object_id,
    runtime_error,
};
use engine_object::ObjectId;
use engine_runtime::RuntimeError;
use std::mem::{align_of, size_of};
use std::ptr;

/// Scene-root enumeration ABI version 1.0.
pub const SCENE_ROOTS_ABI_VERSION: u32 = 0x0001_0000;

/// One root GameObject identity and its mandatory Transform identity.
#[repr(C)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SceneRootIdentity {
    /// The owning GameObject identity.
    pub game_object: i64,
    /// The mandatory Transform identity.
    pub transform: i64,
}

/// Return the independent Scene Roots ABI version.
// SAFETY: unique symbol in the reserved namespace; fixed-width scalar result.
#[unsafe(no_mangle)]
pub extern "C" fn ou_scene_roots_abi_version() -> u32 {
    SCENE_ROOTS_ABI_VERSION
}

/// Return a live GameObject's scene handle, or zero when the object is unowned.
// SAFETY: unique symbol with fixed-width integer arguments and result.
#[unsafe(no_mangle)]
pub extern "C" fn ou_gameobject_scene(runtime: i64, game_object: i64) -> i64 {
    with_runtime(runtime, |runtime| {
        runtime.gameobject_scene(object_id(game_object)?)
    })
    .and_then(|result| result.map_err(runtime_error))
    .map_or_else(i64::from, |scene| scene.map_or(0, ObjectId::to_raw))
}

/// Return the count of live root GameObjects in one published scene.
// SAFETY: unique symbol with fixed-width integer arguments and result.
#[unsafe(no_mangle)]
pub extern "C" fn ou_scene_root_count(runtime: i64, scene: i64) -> i32 {
    with_runtime(runtime, |runtime| {
        runtime.scene_root_identities(scene_id(scene)?)
    })
    .and_then(|result| result.map_err(runtime_error))
    .and_then(|roots| i32::try_from(roots.len()).map_err(|_| COUNT_OVERFLOW))
    .unwrap_or_else(|error| error)
}

/// Copy one complete root snapshot to a caller-owned buffer.
///
/// # Safety
/// `output` must point to writable, naturally aligned storage for `capacity`
/// `SceneRootIdentity` values for the duration of this call. The storage must
/// not overlap memory that the runtime mutates while preparing the snapshot.
// SAFETY: unique symbol; caller owns the output storage and no pointer is retained.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ou_scene_roots_read(
    runtime: i64,
    scene: i64,
    output: *mut SceneRootIdentity,
    capacity: i32,
) -> i32 {
    let Some(capacity) = checked_capacity(output, capacity) else {
        return INVALID_BUFFER;
    };

    let roots = match with_runtime(runtime, |runtime| {
        runtime.scene_root_identities(scene_id(scene)?)
    }) {
        Ok(Ok(roots)) => roots,
        Ok(Err(error)) => return runtime_error(error),
        Err(error) => return error,
    };

    let mut identities = Vec::new();
    if identities.try_reserve_exact(roots.len()).is_err() {
        return ALLOCATION_FAILURE;
    }
    identities.extend(
        roots
            .iter()
            .map(|(game_object, transform)| SceneRootIdentity {
                game_object: game_object.to_raw(),
                transform: transform.to_raw(),
            }),
    );

    let Ok(count) = i32::try_from(roots.len()) else {
        return COUNT_OVERFLOW;
    };
    if capacity < roots.len() {
        return INSUFFICIENT_CAPACITY;
    }
    if roots.is_empty() {
        return 0;
    }

    // SAFETY: checked capacity covers the complete vector, and the caller
    // guarantees a live writable buffer for the declared number of entries.
    unsafe { ptr::copy_nonoverlapping(identities.as_ptr(), output, identities.len()) };
    count
}

fn checked_capacity(output: *mut SceneRootIdentity, capacity: i32) -> Option<usize> {
    let capacity = usize::try_from(capacity).ok()?;
    let byte_count = capacity.checked_mul(size_of::<SceneRootIdentity>())?;

    if output.is_null() {
        return (capacity == 0).then_some(0);
    }

    let alignment_mask = align_of::<SceneRootIdentity>() - 1;
    if (output as usize) & alignment_mask != 0 {
        return None;
    }
    (output as usize).checked_add(byte_count)?;
    Some(capacity)
}

fn scene_id(raw: i64) -> Result<ObjectId, RuntimeError> {
    ObjectId::from_raw(raw).ok_or(RuntimeError::InvalidScene)
}
