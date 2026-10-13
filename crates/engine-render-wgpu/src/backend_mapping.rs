//! Translation between backend-owned WGPU values and public render contracts.

use engine_render_api::{AdapterIdentity, AdapterPolicy, RenderError};

use crate::device::{
    AdapterPolicy as DeviceAdapterPolicy, DeviceError, ScopedGpuClass, ScopedGpuDiagnostics,
};
use crate::readback::ReadbackError;

impl From<DeviceError> for RenderError {
    fn from(error: DeviceError) -> Self {
        map_device_error(error)
    }
}

pub(crate) fn map_device_error(error: DeviceError) -> RenderError {
    match error {
        DeviceError::AdapterUnavailable => RenderError::AdapterUnavailable,
        DeviceError::SurfaceCreation(_) => RenderError::SurfaceCreation,
        DeviceError::SoftwareAdapterRejected | DeviceError::UnsupportedTargetFormat(_) => {
            RenderError::UnsupportedCapabilities
        }
        DeviceError::DeviceRequest(_) => RenderError::DeviceRequest,
        DeviceError::DeviceLost(_)
        | DeviceError::UncapturedGpuError(_)
        | DeviceError::FailureStatePoisoned => RenderError::DeviceLost,
        DeviceError::InvalidTimeout
        | DeviceError::SubmissionTimeout
        | DeviceError::WrongSubmissionIndex(_, _) => RenderError::SubmissionTimeout,
        DeviceError::ScopedGpuError(_) => RenderError::Allocation,
        DeviceError::ReadbackFailure(_) => RenderError::Readback,
        DeviceError::ScopedGpuDiagnostics(diagnostics) => map_scoped_diagnostics(&diagnostics),
        DeviceError::OperationAndScopedGpuError(operation, _) => map_device_error(*operation),
        DeviceError::RecordedFailureAndOperation { recorded, .. }
        | DeviceError::RecordedFailureAndScopedGpuError { recorded, .. } => {
            map_device_error(*recorded)
        }
    }
}

fn map_scoped_diagnostics(diagnostics: &ScopedGpuDiagnostics) -> RenderError {
    match diagnostics.class() {
        ScopedGpuClass::Validation | ScopedGpuClass::ValidationAndAllocation => {
            RenderError::Pipeline
        }
        ScopedGpuClass::Allocation => RenderError::Allocation,
    }
}

pub(crate) fn device_policy(policy: AdapterPolicy) -> DeviceAdapterPolicy {
    match policy {
        AdapterPolicy::Hardware => DeviceAdapterPolicy::Hardware,
        AdapterPolicy::Software => DeviceAdapterPolicy::Software,
    }
}

pub(crate) fn adapter_identity(info: &wgpu::AdapterInfo) -> AdapterIdentity {
    AdapterIdentity::new(
        format!("{:?}", info.backend),
        info.name.clone(),
        info.vendor.to_string(),
        info.device.to_string(),
        info.driver.clone(),
        info.driver_info.clone(),
        format!("{:?}", info.device_type),
    )
}

pub(crate) fn map_readback_error(error: ReadbackError) -> DeviceError {
    match error {
        ReadbackError::SubmissionTimeout => DeviceError::SubmissionTimeout,
        other => DeviceError::ReadbackFailure(format!("{other:?}")),
    }
}
