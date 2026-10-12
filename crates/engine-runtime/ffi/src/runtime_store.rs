//! Private process-wide runtime ownership and staging-aware serialized access.

use crate::status::{
    IDENTITY_EXHAUSTED, INVALID_RUNTIME, INVALID_SCENE, STAGING_BUSY, STORE_UNAVAILABLE,
};
use engine_object::{ObjectId, Registry};
use engine_runtime::Runtime;
use std::sync::{Mutex, OnceLock};

static RUNTIMES: OnceLock<Mutex<Registry<Runtime>>> = OnceLock::new();

pub(crate) fn store() -> &'static Mutex<Registry<Runtime>> {
    RUNTIMES.get_or_init(|| Mutex::new(Registry::default()))
}

pub(crate) fn with_runtime<T>(raw: i64, f: impl FnOnce(&mut Runtime) -> T) -> Result<T, i32> {
    let id = ObjectId::from_raw(raw).ok_or(INVALID_RUNTIME)?;
    let mut runtimes = store().lock().map_err(|_| STORE_UNAVAILABLE)?;
    let runtime = runtimes.get_mut(id).ok_or(INVALID_RUNTIME)?;
    if runtime.scene_staging() {
        return Err(STAGING_BUSY);
    }
    Ok(f(runtime))
}

/// Resolve a runtime and scene for snapshot calls in contract-defined status order.
///
/// The callback owns snapshot-specific staging checks after scene validity is known.
pub(crate) fn with_runtime_scene_snapshot<T>(
    raw_runtime: i64,
    raw_scene: i64,
    f: impl FnOnce(&mut Runtime, ObjectId) -> T,
) -> Result<T, i32> {
    let runtime_id = ObjectId::from_raw(raw_runtime).ok_or(INVALID_RUNTIME)?;
    let mut runtimes = store().lock().map_err(|_| STORE_UNAVAILABLE)?;
    let runtime = runtimes.get_mut(runtime_id).ok_or(INVALID_RUNTIME)?;
    let scene_id = ObjectId::from_raw(raw_scene).ok_or(INVALID_SCENE)?;

    Ok(f(runtime, scene_id))
}

/// Create a context, returning a positive identity or a negative error code.
// SAFETY: unique symbol in the reserved ABI namespace; fixed-width integer interface.
#[unsafe(no_mangle)]
pub extern "C" fn ou_runtime_create() -> i64 {
    match store().lock() {
        Ok(mut runtimes) => runtimes
            .insert(Runtime::default())
            .map(ObjectId::to_raw)
            .unwrap_or(i64::from(IDENTITY_EXHAUSTED)),
        Err(_) => i64::from(STORE_UNAVAILABLE),
    }
}

/// Destroy the context and its objects, returning zero or a negative error code.
// SAFETY: unique symbol in the reserved ABI namespace; fixed-width integer interface.
#[unsafe(no_mangle)]
pub extern "C" fn ou_runtime_destroy(runtime: i64) -> i32 {
    let Some(id) = ObjectId::from_raw(runtime) else {
        return INVALID_RUNTIME;
    };
    match store().lock() {
        Ok(mut runtimes) => {
            if runtimes.get(id).is_some_and(Runtime::scene_staging) {
                return STAGING_BUSY;
            }
            if runtimes.remove(id).is_some() {
                0
            } else {
                INVALID_RUNTIME
            }
        }
        Err(_) => STORE_UNAVAILABLE,
    }
}
