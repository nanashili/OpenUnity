use crate::HostError;
use crate::abi::{
    Close, GetDelegate, Initialize, LOAD_ASSEMBLY_DELEGATE, LoadAssembly, Parameters,
};
use crate::platform::HostPaths;
use libloading::Library;
use std::ffi::c_void;
use std::io::Write;
use std::mem::ManuallyDrop;
use std::path::Path;
use std::ptr;

struct Context {
    raw: *mut c_void,
    close: Close,
}

impl Context {
    fn close(&mut self) -> Result<(), HostError> {
        if self.raw.is_null() {
            return Ok(());
        }
        // SAFETY: this context is uniquely owned and hostfxr has not closed it.
        let status = unsafe { (self.close)(self.raw) };
        if status == 0 {
            self.raw = ptr::null_mut();
            Ok(())
        } else {
            Err(HostError::status("hostfxr_close", status))
        }
    }

    fn finish<T>(mut self, operation: Result<T, HostError>) -> Result<T, HostError> {
        match (operation, self.close()) {
            (Ok(value), Ok(())) => Ok(value),
            (Err(primary), Ok(())) => Err(primary),
            (Ok(_), Err(cleanup)) => Err(cleanup),
            (Err(primary), Err(cleanup)) => Err(primary.with_cleanup(cleanup)),
        }
    }
}

impl Drop for Context {
    fn drop(&mut self) {
        if !self.raw.is_null() {
            // SAFETY: error-path cleanup of this uniquely owned initialized context.
            let status = unsafe { (self.close)(self.raw) };
            if status != 0 {
                // Cleanup diagnostics are best effort: stderr failure must not
                // replace an initiating failure or panic during unwinding.
                let _ = writeln!(
                    std::io::stderr().lock(),
                    "hostfxr context cleanup failed: {status}"
                );
            }
        }
    }
}

struct HostFxr {
    library: ManuallyDrop<Library>,
    initialize: Initialize,
    get_delegate: GetDelegate,
    close: Close,
}

impl HostFxr {
    // The library remains process-resident on both success and setup failure.
    // SAFETY: caller must supply trusted hostfxr implementing the pinned public ABI.
    unsafe fn load(hostfxr: &Path) -> Result<Self, HostError> {
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
        Ok(Self {
            library,
            initialize,
            get_delegate,
            close,
        })
    }

    fn open(&self, paths: &HostPaths) -> Result<Context, HostError> {
        let parameters = Parameters {
            size: std::mem::size_of::<Parameters>(),
            host_path: ptr::null(),
            dotnet_root: paths.root.as_ptr(),
        };
        let mut context = ptr::null_mut();
        // SAFETY: NUL-terminated platform strings and out storage live through the call.
        let status = unsafe { (self.initialize)(paths.config.as_ptr(), &parameters, &mut context) };
        let context = Context {
            raw: context,
            close: self.close,
        };
        // 2 means DifferentRuntimeProperties: reject a configuration mismatch.
        if !matches!(status, 0 | 1) {
            return context.finish(Err(HostError::status("initialize runtime config", status)));
        }
        if context.raw.is_null() {
            return context.finish(Err(HostError::setup(
                "initialize runtime config",
                "null context",
            )));
        }
        Ok(context)
    }
}

impl Context {
    fn runtime_delegate(&self, get_delegate: GetDelegate) -> Result<*mut c_void, HostError> {
        let mut delegate = ptr::null_mut();
        // SAFETY: valid context, documented enum 5, and writable output storage.
        let status = unsafe { get_delegate(self.raw, LOAD_ASSEMBLY_DELEGATE, &mut delegate) };
        if status != 0 {
            return Err(HostError::status("get runtime delegate", status));
        }
        if delegate.is_null() {
            return Err(HostError::setup("get runtime delegate", "null delegate"));
        }
        Ok(delegate)
    }
}

// SAFETY: caller must supply a trusted hostfxr implementing the pinned public ABI.
pub(super) unsafe fn load_runtime(
    hostfxr: &Path,
    paths: &HostPaths,
) -> Result<(ManuallyDrop<Library>, LoadAssembly), HostError> {
    // SAFETY: forwards the caller's trusted host library guarantee.
    let host = unsafe { HostFxr::load(hostfxr) }?;
    let context = host.open(paths)?;
    let result = context.runtime_delegate(host.get_delegate);
    let delegate = context.finish(result)?;
    // SAFETY: documented delegate enum 5 supplies this exact public host signature.
    let load = unsafe { std::mem::transmute::<*mut c_void, LoadAssembly>(delegate) };
    Ok((host.library, load))
}
