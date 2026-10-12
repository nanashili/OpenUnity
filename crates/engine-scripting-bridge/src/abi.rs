#[cfg(not(windows))]
use std::ffi::c_char;
use std::ffi::c_void;

#[cfg(windows)]
pub(super) type HostChar = u16;
#[cfg(not(windows))]
pub(super) type HostChar = c_char;

pub(super) type Initialize =
    unsafe extern "C" fn(*const HostChar, *const Parameters, *mut *mut c_void) -> i32;
pub(super) type GetDelegate = unsafe extern "C" fn(*mut c_void, i32, *mut *mut c_void) -> i32;
pub(super) type Close = unsafe extern "C" fn(*mut c_void) -> i32;
pub(super) type LoadAssembly = unsafe extern "system" fn(
    *const HostChar,
    *const HostChar,
    *const HostChar,
    *const HostChar,
    *mut c_void,
    *mut *mut c_void,
) -> i32;
pub(super) type Version = unsafe extern "C" fn() -> u32;
pub(super) type Create = unsafe extern "C" fn(*const u8, *const u8) -> i64;
pub(super) type CreateScene = unsafe extern "C" fn(*const u8, *const u8, i32) -> i64;
pub(super) type CreateTimedScene = unsafe extern "C" fn(*const u8, *const u8, i32, i32, f64) -> i64;
pub(super) type Invoke = unsafe extern "C" fn(i64, i32, i64, *mut i64) -> i32;
pub(super) type Destroy = unsafe extern "C" fn(i64) -> i32;
pub(super) type SceneSnapshot = unsafe extern "C" fn(i64, *const i64, i32, *mut u8, i32) -> i32;

pub(super) const ABI_VERSION: u32 = 0x0001_0000;
pub(super) const SNAPSHOT_VERSION: u32 = 0x0001_0000;
pub(super) const SNAPSHOT_ROW_STRIDE: u32 = 64;
pub(super) const LOAD_ASSEMBLY_DELEGATE: i32 = 5;
pub(super) const BRIDGE_TYPE: &str = "OpenUnity.Hosting.EntryPoints, OpenUnity.ManagedHosting";

#[repr(C)]
pub(super) struct Parameters {
    pub(super) size: usize,
    pub(super) host_path: *const HostChar,
    pub(super) dotnet_root: *const HostChar,
}
