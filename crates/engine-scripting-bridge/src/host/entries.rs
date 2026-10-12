use super::HostMode;
use crate::HostError;
use crate::abi::{
    ABI_VERSION, Create, CreateScene, CreateTimedScene, Destroy, HostChar, Invoke, LoadAssembly,
    Version,
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
}

impl ManagedEntries {
    // SAFETY: caller guarantees the bridge implements all selected reviewed signatures.
    pub(super) unsafe fn resolve(
        load: LoadAssembly,
        paths: &HostPaths,
        mode: HostMode,
    ) -> Result<Self, HostError> {
        let entries = ManagedEntryLoader { load, paths };
        // SAFETY: caller guarantees every selected managed version signature.
        unsafe { entries.require_version("Version", "managed ABI version") }?;
        let capability = match mode {
            HostMode::Gameplay => Some("GameplayVersion"),
            HostMode::Scene => Some("SceneVersion"),
            HostMode::TimedScene => Some("TimedSceneVersion"),
            HostMode::Script => None,
        };
        if let Some(capability) = capability {
            // SAFETY: caller guarantees the selected controller's exact version signature.
            unsafe { entries.require_version(capability, "controller ABI version") }?;
        }
        let (create, create_scene) = if mode == HostMode::Scene {
            // SAFETY: caller guarantees the exact reviewed three-argument scene signature.
            (
                unsafe { std::mem::transmute::<*mut c_void, Create>(entries.pointer("Create")?) },
                Some(unsafe {
                    std::mem::transmute::<*mut c_void, CreateScene>(entries.pointer("CreateScene")?)
                }),
            )
        } else {
            let name = if mode == HostMode::Gameplay {
                "CreateGameplay"
            } else {
                "Create"
            };
            // SAFETY: both legacy creation entries have the same exact Cdecl signature.
            (
                unsafe { std::mem::transmute::<*mut c_void, Create>(entries.pointer(name)?) },
                None,
            )
        };
        let create_timed_scene = if mode == HostMode::TimedScene {
            // SAFETY: caller guarantees the reviewed five-argument timed-scene ABI.
            Some(unsafe {
                std::mem::transmute::<*mut c_void, CreateTimedScene>(
                    entries.pointer("CreateTimedScene")?,
                )
            })
        } else {
            None
        };
        // SAFETY: reviewed Invoke signature; managed code remains process-resident.
        let invoke =
            unsafe { std::mem::transmute::<*mut c_void, Invoke>(entries.pointer("Invoke")?) };
        // SAFETY: reviewed Destroy signature; managed code remains process-resident.
        let destroy =
            unsafe { std::mem::transmute::<*mut c_void, Destroy>(entries.pointer("Destroy")?) };
        Ok(ManagedEntries {
            create,
            create_scene,
            create_timed_scene,
            invoke,
            destroy,
        })
    }
}
