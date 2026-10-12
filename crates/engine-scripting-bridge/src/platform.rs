use crate::HostError;
use crate::abi::{BRIDGE_TYPE, HostChar};
#[cfg(not(windows))]
use std::ffi::c_char;
use std::path::Path;

pub(super) fn absolute_file(path: &Path) -> Result<(), HostError> {
    if !path.is_absolute() || !path.is_file() {
        return Err(HostError::setup(
            "path",
            format!("expected an absolute existing file: {}", path.display()),
        ));
    }
    Ok(())
}

pub(super) fn host_string(value: &str) -> Result<Vec<HostChar>, HostError> {
    if value.contains('\0') {
        return Err(HostError::setup("host string", "embedded NUL"));
    }
    #[cfg(windows)]
    let text = value.encode_utf16().chain(Some(0)).collect();
    #[cfg(not(windows))]
    let text = value.bytes().map(|b| b as c_char).chain(Some(0)).collect();
    Ok(text)
}

pub(super) fn host_path(path: &Path) -> Result<Vec<HostChar>, HostError> {
    host_string(
        path.to_str()
            .ok_or_else(|| HostError::setup("host path", "path is not Unicode"))?,
    )
}

pub(super) struct HostPaths {
    pub(super) config: Vec<HostChar>,
    pub(super) root: Vec<HostChar>,
    pub(super) assembly: Vec<HostChar>,
    pub(super) type_name: Vec<HostChar>,
}

impl HostPaths {
    pub(super) fn validate(
        dotnet_root: &Path,
        hostfxr: &Path,
        bridge: &Path,
        runtime_config: &Path,
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
        Ok(HostPaths {
            config,
            root,
            assembly,
            type_name,
        })
    }
}
