use super::HostMode;
use crate::HostError;
use crate::abi::{
    ABI_VERSION, Create, CreateScene, CreateTimedScene, Destroy, HostChar, Invoke, LoadAssembly,
    SceneSnapshot, Version,
};
use crate::platform::{HostPaths, host_string};
use std::ffi::c_void;
use std::ptr;

struct ManagedEntryLoader<'a> {
    load: LoadAssembly,
    paths: &'a HostPaths,
}

impl ManagedEntryLoader<'_> {
    fn pointer(&self, name: &str) -> Result<*mut c_void, HostError> {
        let method = host_string(name)?;
        let mut pointer = ptr::null_mut();
        // SAFETY: valid strings, reserved null, and documented -1 UCO sentinel.
        let status = unsafe {
            (self.load)(
                self.paths.assembly.as_ptr(),
                self.paths.type_name.as_ptr(),
                method.as_ptr(),
                (-1_isize) as *const HostChar,
                ptr::null_mut(),
                &mut pointer,
            )
        };
        if status != 0 {
            return Err(HostError::status("load managed entrypoint", status));
        }
        if pointer.is_null() {
            return Err(HostError::setup(
                "load managed entrypoint",
                "null function pointer",
            ));
        }
        Ok(pointer)
    }

    // SAFETY: the selected entry must implement the reviewed Cdecl version signature.
    unsafe fn require_version(&self, name: &str, operation: &'static str) -> Result<(), HostError> {
        // SAFETY: caller guarantees this exact entrypoint signature.
        let version = unsafe { std::mem::transmute::<*mut c_void, Version>(self.pointer(name)?) };
        // SAFETY: the resolved entry has the reviewed version signature.
        let actual = unsafe { version() };
        if actual != ABI_VERSION {
            return Err(HostError::setup(
                operation,
                format!("expected {ABI_VERSION}, got {actual}"),
            ));
        }
        Ok(())
    }
}

pub(super) struct ManagedEntries {
    pub(super) create: Create,
    pub(super) create_scene: Option<CreateScene>,
    pub(super) create_timed_scene: Option<CreateTimedScene>,
    pub(super) invoke: Invoke,
    pub(super) destroy: Destroy,
    pub(super) scene_snapshot: Option<SceneSnapshot>,
}

impl ManagedEntries {
    // SAFETY: caller guarantees the bridge implements all selected reviewed signatures.
    pub(super) unsafe fn resolve(
        load: LoadAssembly,
        paths: &HostPaths,
        mode: HostMode,
    ) -> Result<Self, HostError> {
        let entries = ManagedEntryLoader { load, paths };
        // SAFETY: caller guarantees the versions selected by this host mode.
        unsafe { require_mode_versions(&entries, mode)? };

        let (create, create_scene) = resolve_creation_entries(&entries, mode)?;
        let create_timed_scene = resolve_timed_scene_entry(&entries, mode)?;
        let (invoke, destroy) = resolve_lifecycle_entries(&entries)?;
        let scene_snapshot = resolve_snapshot_entry(&entries, mode)?;

        Ok(ManagedEntries {
            create,
            create_scene,
            create_timed_scene,
            invoke,
            destroy,
            scene_snapshot,
        })
    }
}

unsafe fn require_mode_versions(
    entries: &ManagedEntryLoader<'_>,
    mode: HostMode,
) -> Result<(), HostError> {
    // SAFETY: the caller guarantees the bridge's reviewed Cdecl version signature.
    unsafe { entries.require_version("Version", "managed ABI version") }?;

    let controller_version = match mode {
        HostMode::Gameplay => Some("GameplayVersion"),
        HostMode::Scene | HostMode::SceneSnapshot => Some("SceneVersion"),
        HostMode::TimedScene | HostMode::TimedSceneSnapshot => Some("TimedSceneVersion"),
        HostMode::Script => None,
    };
    if let Some(version_name) = controller_version {
        // SAFETY: the caller guarantees the selected controller's reviewed version signature.
        unsafe { entries.require_version(version_name, "controller ABI version") }?;
    }

    Ok(())
}

fn resolve_creation_entries(
    entries: &ManagedEntryLoader<'_>,
    mode: HostMode,
) -> Result<(Create, Option<CreateScene>), HostError> {
    let is_scene_mode = matches!(mode, HostMode::Scene | HostMode::SceneSnapshot);
    let create_name = if mode == HostMode::Gameplay {
        "CreateGameplay"
    } else {
        "Create"
    };

    // SAFETY: every creation mode uses the reviewed Cdecl `Create` signature.
    let create =
        unsafe { std::mem::transmute::<*mut c_void, Create>(entries.pointer(create_name)?) };
    let create_scene = if is_scene_mode {
        // SAFETY: scene modes select the reviewed three-argument Cdecl scene signature.
        Some(unsafe {
            std::mem::transmute::<*mut c_void, CreateScene>(entries.pointer("CreateScene")?)
        })
    } else {
        None
    };

    Ok((create, create_scene))
}

fn resolve_timed_scene_entry(
    entries: &ManagedEntryLoader<'_>,
    mode: HostMode,
) -> Result<Option<CreateTimedScene>, HostError> {
    if !matches!(mode, HostMode::TimedScene | HostMode::TimedSceneSnapshot) {
        return Ok(None);
    }

    // SAFETY: timed-scene modes select the reviewed five-argument Cdecl ABI.
    let create = unsafe {
        std::mem::transmute::<*mut c_void, CreateTimedScene>(entries.pointer("CreateTimedScene")?)
    };
    Ok(Some(create))
}

fn resolve_lifecycle_entries(
    entries: &ManagedEntryLoader<'_>,
) -> Result<(Invoke, Destroy), HostError> {
    // SAFETY: the reviewed entries use Cdecl and managed code remains process-resident.
    let invoke = unsafe { std::mem::transmute::<*mut c_void, Invoke>(entries.pointer("Invoke")?) };
    // SAFETY: the reviewed entries use Cdecl and managed code remains process-resident.
    let destroy =
        unsafe { std::mem::transmute::<*mut c_void, Destroy>(entries.pointer("Destroy")?) };

    Ok((invoke, destroy))
}

fn resolve_snapshot_entry(
    entries: &ManagedEntryLoader<'_>,
    mode: HostMode,
) -> Result<Option<SceneSnapshot>, HostError> {
    if !matches!(mode, HostMode::SceneSnapshot | HostMode::TimedSceneSnapshot) {
        return Ok(None);
    }

    // SAFETY: snapshot modes require the reviewed Cdecl snapshot-version UCO.
    unsafe { entries.require_version("SceneSnapshotVersion", "scene snapshot ABI version") }?;

    // SAFETY: the version gate precedes resolution of this exact reviewed Cdecl capture UCO.
    let capture = unsafe {
        std::mem::transmute::<*mut c_void, SceneSnapshot>(entries.pointer("CaptureSceneSnapshot")?)
    };
    Ok(Some(capture))
}
