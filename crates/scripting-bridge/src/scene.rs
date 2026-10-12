//! Audited cdecl scene boundary. Payloads are copied/validated synchronously.
use super::{
    INVALID_BUFFER, INVALID_RUNTIME, Runtime, RuntimeError, object_id, runtime_error, store,
};
use openunity_object_model::ObjectId;

/// Invalid, stale, or foreign scene handle.
pub const INVALID_SCENE: i32 = -15;
/// Invalid or unsupported scene plan.
pub const INVALID_SCENE_PLAN: i32 = -16;
/// Busy/terminal transaction, owner thread violation, or forbidden legacy call.
pub const STAGING_BUSY: i32 = -17;
const MAX_PLAN_BYTES: usize = 16 * 1024 * 1024;

fn trusted<T>(raw: i64, f: impl FnOnce(&mut Runtime) -> Result<T, RuntimeError>) -> Result<T, i32> {
    let id = ObjectId::from_raw(raw).ok_or(INVALID_RUNTIME)?;
    let mut runtimes = store().lock().map_err(|_| super::STORE_UNAVAILABLE)?;
    f(runtimes.get_mut(id).ok_or(INVALID_RUNTIME)?).map_err(runtime_error)
}
fn scene_id(raw: i64) -> Result<ObjectId, RuntimeError> {
    ObjectId::from_raw(raw).ok_or(RuntimeError::InvalidScene)
}

/// Return independent scene capability version 1.0.
// SAFETY: one definition in reserved ou_ namespace; fixed-width scalar interface.
#[unsafe(no_mangle)]
pub extern "C" fn ou_scene_abi_version() -> u32 {
    0x0001_0000
}

/// Validate and stage an owned JSON scene plan without publishing objects.
/// # Safety
/// `plan` must be readable for `byte_count` bytes throughout this call.
// SAFETY: unique reserved symbol; raw byte input is bounded and never retained.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ou_scene_stage(runtime: i64, plan: *const u8, byte_count: i32) -> i64 {
    let Ok(count) = usize::try_from(byte_count) else {
        return i64::from(INVALID_BUFFER);
    };
    if count == 0 || count > MAX_PLAN_BYTES || plan.is_null() {
        return i64::from(INVALID_BUFFER);
    }
    // SAFETY: checked non-null bounded count; caller guarantees readable byte storage.
    let bytes = unsafe { std::slice::from_raw_parts(plan, count) };
    let Ok(plan) = openunity_scene::parse_plan(bytes) else {
        return i64::from(INVALID_SCENE_PLAN);
    };
    trusted(runtime, |runtime| runtime.scene_stage(&plan))
        .map(ObjectId::to_raw)
        .unwrap_or_else(i64::from)
}

/// Resolve a file-local identity through the trusted staging route.
// SAFETY: unique reserved symbol; fixed-width integer arguments/results.
#[unsafe(no_mangle)]
pub extern "C" fn ou_scene_object(runtime: i64, scene: i64, local_file_id: i64) -> i64 {
    trusted(runtime, |runtime| {
        runtime.scene_object(scene_id(scene)?, local_file_id)
    })
    .map(ObjectId::to_raw)
    .unwrap_or_else(i64::from)
}

macro_rules! operation {
    ($export:ident, $method:ident, $description:literal) => {
        #[doc = $description]
        // SAFETY: unique reserved symbol; fixed-width integer interface.
        #[unsafe(no_mangle)]
        pub extern "C" fn $export(runtime: i64, scene: i64) -> i32 {
            trusted(runtime, |runtime| runtime.$method(scene_id(scene)?))
                .map_or_else(|error| error, |()| 0)
        }
    };
}
operation!(
    ou_scene_publish,
    scene_publish,
    "Atomically publish a staged scene without invoking callbacks."
);
operation!(
    ou_scene_abort,
    scene_abort,
    "Consume an unpublished scene without lifecycle callbacks."
);
operation!(
    ou_scene_destroy,
    scene_destroy,
    "Queue deferred destruction of every scene-owned GameObject once."
);
operation!(
    ou_scene_release,
    scene_release,
    "Silently consume published scene resources, including after callback fault."
);

/// Return native hierarchy sibling order, including ordered scene roots.
// SAFETY: unique reserved symbol; fixed-width integer interface.
#[unsafe(no_mangle)]
pub extern "C" fn ou_transform_sibling_index(runtime: i64, transform: i64) -> i32 {
    super::with_runtime(runtime, |runtime| {
        runtime.transform_sibling_index(object_id(transform)?)
    })
    .and_then(|result| result.map_err(runtime_error))
    .unwrap_or_else(|error| error)
}
