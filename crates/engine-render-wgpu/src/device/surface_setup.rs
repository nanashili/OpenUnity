//! Device and surface creation for an owned native window handle.

use super::creation::{create_instance, create_with_adapter, request_adapter};
use super::{AdapterPolicy, DeviceError, DeviceOwner};

/// Create a surface-compatible device while retaining the safe window target.
pub(crate) fn create_for_window<W>(
    window: W,
    policy: AdapterPolicy,
) -> Result<(DeviceOwner, wgpu::Surface<'static>), DeviceError>
where
    W: wgpu::WindowHandle + 'static,
{
    let instance = create_instance();
    let surface = instance
        .create_surface(window)
        .map_err(|error| DeviceError::SurfaceCreation(error.to_string()))?;
    let adapter = request_adapter(&instance, Some(&surface), policy)?;
    let device = create_with_adapter(instance, adapter, policy)?;

    Ok((device, surface))
}
