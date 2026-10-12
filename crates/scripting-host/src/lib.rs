//! Rust-driven .NET hosting for the reviewed script and gameplay host ABIs.
//!
//! This does not implement Unity APIs. Only the selected hostfxr and reviewed
//! managed bridge may supply native function pointers. CoreCLR and its libraries
//! remain process-resident; closing a script releases its session, not the runtime.

use libloading::Library;
use std::error::Error;
use std::ffi::{CString, c_char, c_void};
use std::fmt;
use std::marker::PhantomData;
use std::mem::ManuallyDrop;
use std::path::Path;
use std::ptr;
use std::rc::Rc;

#[cfg(windows)]
type HostChar = u16;
#[cfg(not(windows))]
type HostChar = c_char;

type Initialize = unsafe extern "C" fn(*const HostChar, *const Parameters, *mut *mut c_void) -> i32;
type GetDelegate = unsafe extern "C" fn(*mut c_void, i32, *mut *mut c_void) -> i32;
type Close = unsafe extern "C" fn(*mut c_void) -> i32;
type LoadAssembly = unsafe extern "system" fn(
    *const HostChar,
    *const HostChar,
    *const HostChar,
    *const HostChar,
    *mut c_void,
    *mut *mut c_void,
) -> i32;
type Version = unsafe extern "C" fn() -> u32;
type Create = unsafe extern "C" fn(*const u8, *const u8) -> i64;
type CreateScene = unsafe extern "C" fn(*const u8, *const u8, i32) -> i64;
type CreateTimedScene = unsafe extern "C" fn(*const u8, *const u8, i32, i32, f64) -> i64;
type Invoke = unsafe extern "C" fn(i64, i32, i64, *mut i64) -> i32;
type Destroy = unsafe extern "C" fn(i64) -> i32;

const ABI_VERSION: u32 = 0x0001_0000;
const LOAD_ASSEMBLY_DELEGATE: i32 = 5;
const BRIDGE_TYPE: &str = "OpenUnity.Hosting.EntryPoints, OpenUnity.ManagedHosting";

#[repr(C)]
struct Parameters {
    size: usize,
    host_path: *const HostChar,
    dotnet_root: *const HostChar,
}

/// A host setup, loading, ABI, or callback failure with operation context.
#[derive(Debug)]
pub struct HostError {
    operation: &'static str,
    detail: String,
    status: Option<i32>,
}

impl HostError {
    fn setup(operation: &'static str, detail: impl ToString) -> Self {
        Self {
            operation,
            detail: detail.to_string(),
            status: None,
        }
    }

    fn status(operation: &'static str, status: i32) -> Self {
        Self {
            operation,
            detail: format!("status {status} (0x{:08x})", status as u32),
            status: Some(status),
        }
    }

    /// Return the native/managed status, if this was an ABI failure.
    pub fn code(&self) -> Option<i32> {
        self.status
    }
}

impl fmt::Display for HostError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.operation, self.detail)
    }
}

impl Error for HostError {}

fn absolute_file(path: &Path) -> Result<(), HostError> {
    if !path.is_absolute() || !path.is_file() {
        return Err(HostError::setup(
            "path",
            format!("expected an absolute existing file: {}", path.display()),
        ));
    }
    Ok(())
}

fn host_string(value: &str) -> Result<Vec<HostChar>, HostError> {
    if value.contains('\0') {
        return Err(HostError::setup("host string", "embedded NUL"));
    }
    #[cfg(windows)]
    let text = value.encode_utf16().chain(Some(0)).collect();
    #[cfg(not(windows))]
    let text = value.bytes().map(|b| b as c_char).chain(Some(0)).collect();
    Ok(text)
}

fn host_path(path: &Path) -> Result<Vec<HostChar>, HostError> {
    host_string(
        path.to_str()
            .ok_or_else(|| HostError::setup("host path", "path is not Unicode"))?,
    )
}

struct Context {
    raw: *mut c_void,
    close: Close,
}

impl Context {
    fn finish(mut self) -> Result<(), HostError> {
        // SAFETY: this non-null context belongs to hostfxr and is closed exactly once.
        let status = unsafe { (self.close)(self.raw) };
        self.raw = ptr::null_mut();
        if status == 0 {
            Ok(())
        } else {
            Err(HostError::status("hostfxr_close", status))
        }
    }
}

impl Drop for Context {
    fn drop(&mut self) {
        if !self.raw.is_null() {
            // SAFETY: error-path cleanup of this uniquely owned initialized context.
            let status = unsafe { (self.close)(self.raw) };
            if status != 0 {
                eprintln!("hostfxr context cleanup failed: {status}");
            }
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum HostMode {
    Script,
    Gameplay,
    Scene,
    TimedScene,
}

/// A process-resident runtime and its reviewed managed bridge.
///
/// Deliberately neither Send nor Sync: this first host dispatches from its owner
/// thread. The managed registry also serializes callbacks. Neither runtime unload
/// nor Unity reload semantics are promised.
pub struct ManagedHost {
    // Pin the host library through process exit, including error paths after init.
    // CoreCLR can remain loaded after hostfxr_close and retain associated code.
    _library: ManuallyDrop<Library>,
    create: Create,
    create_scene: Option<CreateScene>,
    create_timed_scene: Option<CreateTimedScene>,
    invoke: Invoke,
    destroy: Destroy,
    mode: HostMode,
    _owner_thread: PhantomData<Rc<()>>,
}

impl ManagedHost {
    /// Initialize hostfxr and obtain the four independently specified delegates.
    ///
    /// # Safety
    /// `hostfxr` must be a trusted .NET host library implementing the public
    /// hostfxr ABI. `bridge` must be the reviewed OpenUnity.ManagedHosting assembly
    /// implementing INFRA-HOST-001 with exactly its documented native signatures.
    /// Loading arbitrary native code or mismatched managed signatures is unsafe.
    pub unsafe fn load(
        dotnet_root: &Path,
        hostfxr: &Path,
        bridge: &Path,
        runtime_config: &Path,
    ) -> Result<Self, HostError> {
        // SAFETY: forwards the caller's trusted host and bridge guarantees.
        unsafe {
            Self::load_mode(
                dotnet_root,
                hostfxr,
                bridge,
                runtime_config,
                HostMode::Script,
            )
        }
    }

    /// Load the independently versioned MonoBehaviour gameplay controller.
    /// Legacy hosted-script clients do not resolve this additional capability.
    ///
    /// # Safety
    /// The paths must satisfy [`Self::load`]'s guarantees. The managed bridge
    /// must additionally implement the exact INFRA-LIFECYCLE-001 Cdecl
    /// GameplayVersion and CreateGameplay signatures.
    pub unsafe fn load_gameplay(
        dotnet_root: &Path,
        hostfxr: &Path,
        bridge: &Path,
        runtime_config: &Path,
    ) -> Result<Self, HostError> {
        // SAFETY: forwards the caller's additional reviewed gameplay ABI guarantee.
        unsafe {
            Self::load_mode(
                dotnet_root,
                hostfxr,
                bridge,
                runtime_config,
                HostMode::Gameplay,
            )
        }
    }

    /// Load the independently versioned saved-scene controller.
    ///
    /// # Safety
    /// Paths must satisfy [`Self::load`] and the bridge must implement the exact
    /// INFRA-SCENE-001 SceneVersion and three-argument CreateScene signatures.
    pub unsafe fn load_scene(
        dotnet_root: &Path,
        hostfxr: &Path,
        bridge: &Path,
        runtime_config: &Path,
    ) -> Result<Self, HostError> {
        // SAFETY: forwards the caller's reviewed scene ABI guarantees.
        unsafe {
            Self::load_mode(
                dotnet_root,
                hostfxr,
                bridge,
                runtime_config,
                HostMode::Scene,
            )
        }
    }

    /// Load the independently versioned timed saved-scene controller.
    ///
    /// # Safety
    /// Paths must satisfy [`Self::load`]. The bridge must additionally implement
    /// INFRA-TIME-001 TimedSceneVersion and the exact five-argument CreateTimedScene ABI.
    pub unsafe fn load_timed_scene(
        dotnet_root: &Path,
        hostfxr: &Path,
        bridge: &Path,
        runtime_config: &Path,
    ) -> Result<Self, HostError> {
        // SAFETY: forwards the caller's reviewed timed-scene signature guarantee.
        unsafe {
            Self::load_mode(
                dotnet_root,
                hostfxr,
                bridge,
                runtime_config,
                HostMode::TimedScene,
            )
        }
    }

    unsafe fn load_mode(
        dotnet_root: &Path,
        hostfxr: &Path,
        bridge: &Path,
        runtime_config: &Path,
        mode: HostMode,
    ) -> Result<Self, HostError> {
        for file in [hostfxr, bridge, runtime_config] {
            absolute_file(file)?;
        }
        if !dotnet_root.is_absolute() || !dotnet_root.is_dir() {
            return Err(HostError::setup(
                "dotnet root",
                "expected an absolute existing directory",
            ));
        }
        let config = host_path(runtime_config)?;
        let root = host_path(dotnet_root)?;
        let assembly = host_path(bridge)?;
        let type_name = host_string(BRIDGE_TYPE)?;
        // SAFETY: caller guarantees a trusted hostfxr library; constructors are trusted.
        let library = ManuallyDrop::new(
            unsafe { Library::new(hostfxr) }.map_err(|e| HostError::setup("load hostfxr", e))?,
        );
        // SAFETY: exact names and signatures from the public .NET 8 hostfxr.h.
        let initialize: Initialize = unsafe {
            *library
                .get(b"hostfxr_initialize_for_runtime_config\0")
                .map_err(|e| HostError::setup("resolve initialize", e))?
        };
        // SAFETY: same public host ABI; library stays pinned.
        let get_delegate: GetDelegate = unsafe {
            *library
                .get(b"hostfxr_get_runtime_delegate\0")
                .map_err(|e| HostError::setup("resolve get delegate", e))?
        };
        // SAFETY: same public host ABI; library stays pinned.
        let close: Close = unsafe {
            *library
                .get(b"hostfxr_close\0")
                .map_err(|e| HostError::setup("resolve close", e))?
        };
        let parameters = Parameters {
            size: std::mem::size_of::<Parameters>(),
            host_path: ptr::null(),
            dotnet_root: root.as_ptr(),
        };
        let mut context = ptr::null_mut();
        // SAFETY: NUL-terminated platform strings and out storage live through the call.
        let status = unsafe { initialize(config.as_ptr(), &parameters, &mut context) };
        let context = Context {
            raw: context,
            close,
        };
        // 2 means DifferentRuntimeProperties: reject a configuration mismatch.
        if !matches!(status, 0 | 1) {
            return Err(HostError::status("initialize runtime config", status));
        }
        if context.raw.is_null() {
            return Err(HostError::setup(
                "initialize runtime config",
                "null context",
            ));
        }
        let mut delegate = ptr::null_mut();
        // SAFETY: valid context, documented enum 5, and writable output storage.
        let status = unsafe { get_delegate(context.raw, LOAD_ASSEMBLY_DELEGATE, &mut delegate) };
        if status != 0 {
            return Err(HostError::status("get runtime delegate", status));
        }
        if delegate.is_null() {
            return Err(HostError::setup("get runtime delegate", "null delegate"));
        }
        context.finish()?;
        // SAFETY: enum 5 returns exactly load_assembly_and_get_function_pointer_fn.
        let load: LoadAssembly = unsafe { std::mem::transmute(delegate) };
        let entry = |name: &str| -> Result<*mut c_void, HostError> {
            let method = host_string(name)?;
            let mut pointer = ptr::null_mut();
            // SAFETY: valid strings, reserved null, and documented -1 UCO sentinel.
            let status = unsafe {
                load(
                    assembly.as_ptr(),
                    type_name.as_ptr(),
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
        };
        // SAFETY: caller guarantees the bridge's reviewed exact UCO Cdecl signatures.
        let version: Version = unsafe { std::mem::transmute(entry("Version")?) };
        // SAFETY: same signature guarantee and process-resident managed code.
        let actual = unsafe { version() };
        if actual != ABI_VERSION {
            return Err(HostError::setup(
                "managed ABI version",
                format!("expected {ABI_VERSION}, got {actual}"),
            ));
        }
        if mode != HostMode::Script {
            let capability = match mode {
                HostMode::Gameplay => "GameplayVersion",
                HostMode::Scene => "SceneVersion",
                HostMode::TimedScene => "TimedSceneVersion",
                HostMode::Script => unreachable!("script capability was excluded"),
            };
            // SAFETY: callers guarantee the reviewed capability signature.
            let version: Version = unsafe { std::mem::transmute(entry(capability)?) };
            // SAFETY: checked trusted entrypoint with the exact version signature.
            let actual = unsafe { version() };
            if actual != ABI_VERSION {
                return Err(HostError::setup(
                    "controller ABI version",
                    format!("expected {ABI_VERSION}, got {actual}"),
                ));
            }
        }
        let (create, create_scene) = if mode == HostMode::Scene {
            // SAFETY: caller guarantees the exact reviewed three-argument scene signature.
            (
                unsafe { std::mem::transmute::<*mut c_void, Create>(entry("Create")?) },
                Some(unsafe {
                    std::mem::transmute::<*mut c_void, CreateScene>(entry("CreateScene")?)
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
                unsafe { std::mem::transmute::<*mut c_void, Create>(entry(name)?) },
                None,
            )
        };
        let create_timed_scene = if mode == HostMode::TimedScene {
            // SAFETY: caller guarantees the reviewed five-argument timed-scene ABI.
            Some(unsafe {
                std::mem::transmute::<*mut c_void, CreateTimedScene>(entry("CreateTimedScene")?)
            })
        } else {
            None
        };
        // SAFETY: reviewed Invoke signature; managed code remains process-resident.
        let invoke = unsafe { std::mem::transmute::<*mut c_void, Invoke>(entry("Invoke")?) };
        // SAFETY: reviewed Destroy signature; managed code remains process-resident.
        let destroy = unsafe { std::mem::transmute::<*mut c_void, Destroy>(entry("Destroy")?) };
        Ok(Self {
            _library: library,
            create,
            create_scene,
            create_timed_scene,
            invoke,
            destroy,
            mode,
            _owner_thread: PhantomData,
        })
    }

    /// Construct a script object implementing the bridge's IHostedScript contract.
    pub fn create_script(
        &self,
        assembly: &Path,
        type_name: &str,
    ) -> Result<ScriptSession<'_>, HostError> {
        if self.mode != HostMode::Script {
            return Err(HostError::setup(
                "create script",
                "host was loaded for another controller",
            ));
        }
        self.create_session(assembly, type_name)
    }

    /// Construct a controller for an ordinary MonoBehaviour gameplay type.
    /// Initialization attaches it to a native-owned root. Requires a host
    /// constructed by [`Self::load_gameplay`].
    pub fn create_gameplay(
        &self,
        assembly: &Path,
        type_name: &str,
    ) -> Result<ScriptSession<'_>, HostError> {
        if self.mode != HostMode::Gameplay {
            return Err(HostError::setup(
                "create gameplay",
                "gameplay capability was not loaded",
            ));
        }
        self.create_session(assembly, type_name)
    }

    /// Create a saved-scene controller from an immutable UTF-8 transfer plan.
    /// Reflection and native graph validation occur before native publication.
    pub fn create_scene(
        &self,
        assembly: &Path,
        plan: &[u8],
    ) -> Result<ScriptSession<'_>, HostError> {
        let create = self
            .create_scene
            .ok_or_else(|| HostError::setup("create scene", "scene capability was not loaded"))?;
        absolute_file(assembly)?;
        if plan.is_empty() || plan.len() > 16 * 1024 * 1024 {
            return Err(HostError::setup("create scene", "plan byte limit"));
        }
        let path = CString::new(
            assembly
                .to_str()
                .ok_or_else(|| HostError::setup("scene assembly", "nonUnicode path"))?,
        )
        .map_err(|e| HostError::setup("scene assembly", e))?;
        // SAFETY: trusted exact entrypoint; both byte buffers live through the call.
        let id = unsafe { create(path.as_ptr().cast(), plan.as_ptr(), plan.len() as i32) };
        if id <= 0 {
            return Err(HostError::status(
                "create scene",
                i32::try_from(id).unwrap_or(-6),
            ));
        }
        Ok(ScriptSession {
            host: self,
            id: Some(id),
        })
    }

    /// Create a timed scene using an explicitly selected native elapsed source.
    /// Source zero requires step zero; source one requires a finite 1us..60s step.
    pub fn create_timed_scene(
        &self,
        assembly: &Path,
        plan: &[u8],
        source: i32,
        step: f64,
    ) -> Result<ScriptSession<'_>, HostError> {
        let create = self.create_timed_scene.ok_or_else(|| {
            HostError::setup(
                "create timed scene",
                "timed scene capability was not loaded",
            )
        })?;
        validate_time_source(source, step)?;
        absolute_file(assembly)?;
        if plan.is_empty() || plan.len() > 16 * 1024 * 1024 {
            return Err(HostError::setup("create timed scene", "plan byte limit"));
        }
        let path = CString::new(
            assembly
                .to_str()
                .ok_or_else(|| HostError::setup("timed scene assembly", "nonUnicode path"))?,
        )
        .map_err(|error| HostError::setup("timed scene assembly", error))?;
        // SAFETY: trusted exact entrypoint and live immutable buffers for this call.
        let id = unsafe {
            create(
                path.as_ptr().cast(),
                plan.as_ptr(),
                plan.len() as i32,
                source,
                step,
            )
        };
        if id <= 0 {
            return Err(HostError::status(
                "create timed scene",
                i32::try_from(id).unwrap_or(-6),
            ));
        }
        Ok(ScriptSession {
            host: self,
            id: Some(id),
        })
    }

    fn create_session(
        &self,
        assembly: &Path,
        type_name: &str,
    ) -> Result<ScriptSession<'_>, HostError> {
        absolute_file(assembly)?;
        let path = CString::new(
            assembly
                .to_str()
                .ok_or_else(|| HostError::setup("script path", "path is not Unicode"))?,
        )
        .map_err(|e| HostError::setup("script path", e))?;
        let name = CString::new(type_name).map_err(|e| HostError::setup("script type", e))?;
        // SAFETY: bridge is trusted; both UTF-8 CStrings remain valid for the call.
        let id = unsafe { (self.create)(path.as_ptr().cast(), name.as_ptr().cast()) };
        if id <= 0 {
            return Err(HostError::status(
                if self.mode == HostMode::Gameplay {
                    "create gameplay"
                } else {
                    "create script"
                },
                i32::try_from(id).unwrap_or(-6),
            ));
        }
        Ok(ScriptSession {
            host: self,
            id: Some(id),
        })
    }
}

/// Validate the reviewed source configuration before import or host creation.
pub fn validate_time_source(source: i32, step: f64) -> Result<(), HostError> {
    let valid = step.is_finite()
        && match source {
            0 => step == 0.0,
            1 => (0.000001..=60.0).contains(&step),
            _ => false,
        };
    if !valid {
        return Err(HostError::setup(
            "time source",
            "invalid source or elapsed step",
        ));
    }
    Ok(())
}

/// An owned managed script session, borrowing its native host.
pub struct ScriptSession<'a> {
    host: &'a ManagedHost,
    id: Option<i64>,
}

impl ScriptSession<'_> {
    fn invoke(&mut self, opcode: i32, value: i64) -> Result<i64, HostError> {
        let mut output = 0_i64;
        // SAFETY: owned live session and aligned output storage valid for the call.
        let status = unsafe {
            (self.host.invoke)(self.id.expect("owned session"), opcode, value, &mut output)
        };
        if status == 0 {
            Ok(output)
        } else {
            Err(HostError::status("managed callback", status))
        }
    }

    /// Initialize this session exactly once.
    pub fn initialize(&mut self, value: i64) -> Result<i64, HostError> {
        self.invoke(1, value)
    }
    /// Invoke one tick after initialization and before shutdown.
    pub fn tick(&mut self, value: i64) -> Result<i64, HostError> {
        self.invoke(2, value)
    }
    /// Invoke explicit script shutdown; this does not destroy the session.
    pub fn shutdown(&mut self, value: i64) -> Result<i64, HostError> {
        self.invoke(3, value)
    }
    /// Release this script's managed references without invoking a callback.
    pub fn close(mut self) -> Result<(), HostError> {
        // SAFETY: consuming self releases its unique handle exactly once.
        let status = unsafe { (self.host.destroy)(self.id.expect("owned session")) };
        if status == 0 {
            self.id = None;
            Ok(())
        } else {
            Err(HostError::status("destroy script", status))
        }
    }
}

impl Drop for ScriptSession<'_> {
    fn drop(&mut self) {
        if let Some(id) = self.id.take() {
            // SAFETY: unique live handle, trusted bridge, host outlives the session.
            let status = unsafe { (self.host.destroy)(id) };
            if status != 0 {
                eprintln!("managed session cleanup failed: {status}");
            }
        }
    }
}

#[cfg(all(test, feature = "conformance"))]
mod conformance;
