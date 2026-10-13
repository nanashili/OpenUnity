//! WGPU instance, adapter policy, and device acquisition.

use std::sync::{Arc, Mutex};

use super::adapter_selection::{AdapterClass, classify_adapter, validate_adapter_target_formats};
use super::{AdapterPolicy, DeviceError, DeviceOwner, FailureState, install_failure_callbacks};

pub(super) fn create_device(policy: AdapterPolicy) -> Result<DeviceOwner, DeviceError> {
    let instance = create_instance();
    let adapter = request_adapter(&instance, None, policy)?;
    create_with_adapter(instance, adapter, policy)
}

pub(super) fn create_with_adapter(
    instance: wgpu::Instance,
    adapter: wgpu::Adapter,
    policy: AdapterPolicy,
) -> Result<DeviceOwner, DeviceError> {
    validate_adapter(&adapter, policy)?;

    let failure = Arc::new(Mutex::new(FailureState { error: None }));
    let descriptor = wgpu::DeviceDescriptor {
        label: Some("openunity-render-device"),
        required_features: wgpu::Features::empty(),
        required_limits: adapter.limits(),
        ..Default::default()
    };
    let (device, queue) = pollster::block_on(adapter.request_device(&descriptor))
        .map_err(|error| DeviceError::DeviceRequest(error.to_string()))?;

    install_failure_callbacks(&device, &failure);
    Ok(DeviceOwner {
        instance,
        adapter,
        device,
        queue,
        failure,
    })
}

pub(super) fn create_instance() -> wgpu::Instance {
    wgpu::Instance::new(&wgpu::InstanceDescriptor {
        backends: wgpu::Backends::all(),
        ..Default::default()
    })
}

pub(super) fn request_adapter<'surface>(
    instance: &wgpu::Instance,
    compatible_surface: Option<&wgpu::Surface<'surface>>,
    policy: AdapterPolicy,
) -> Result<wgpu::Adapter, DeviceError> {
    let options = wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::HighPerformance,
        force_fallback_adapter: matches!(policy, AdapterPolicy::Software),
        compatible_surface,
    };
    pollster::block_on(instance.request_adapter(&options))
        .map_err(|_| DeviceError::AdapterUnavailable)
}

fn validate_adapter(adapter: &wgpu::Adapter, policy: AdapterPolicy) -> Result<(), DeviceError> {
    let adapter_info = adapter.get_info();
    let class =
        classify_adapter(adapter_info.device_type).ok_or(DeviceError::SoftwareAdapterRejected)?;
    let expected = match policy {
        AdapterPolicy::Hardware => AdapterClass::Hardware,
        AdapterPolicy::Software => AdapterClass::Software,
    };
    if class != expected {
        return Err(DeviceError::SoftwareAdapterRejected);
    }

    validate_adapter_target_formats(adapter).map_err(DeviceError::UnsupportedTargetFormat)
}
