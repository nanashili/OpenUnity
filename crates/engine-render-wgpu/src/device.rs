//! WGPU instance, adapter, device, and queue owner.

use std::sync::{Arc, Mutex};
use std::time::Duration;

#[path = "adapter_selection.rs"]
mod adapter_selection;

#[path = "scoped_diagnostics.rs"]
mod scoped_diagnostics;

#[path = "error_reconciliation.rs"]
mod error_reconciliation;

use adapter_selection::{AdapterClass, classify_adapter, validate_adapter_target_formats};
use error_reconciliation::reconcile_error_state;

pub(crate) use scoped_diagnostics::{ScopedGpuClass, ScopedGpuDiagnostics};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum AdapterPolicy {
    Hardware,
    Software,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum DeviceError {
    AdapterUnavailable,
    SoftwareAdapterRejected,
    UnsupportedTargetFormat(wgpu::TextureFormat),
    InvalidTimeout,
    DeviceRequest(String),
    DeviceLost(String),
    UncapturedGpuError(String),
    ScopedGpuError(String),
    ScopedGpuDiagnostics(ScopedGpuDiagnostics),
    OperationAndScopedGpuError(Box<Self>, ScopedGpuDiagnostics),
    RecordedFailureAndOperation {
        recorded: Box<Self>,
        operation: Box<Self>,
    },
    RecordedFailureAndScopedGpuError {
        recorded: Box<Self>,
        operation: Option<Box<Self>>,
        diagnostics: ScopedGpuDiagnostics,
    },
    SubmissionTimeout,
    ReadbackFailure(String),
    WrongSubmissionIndex(u64, u64),
    FailureStatePoisoned,
}

#[derive(Clone, Debug)]
struct FailureState {
    error: Option<DeviceError>,
}

pub(crate) struct DeviceOwner {
    instance: wgpu::Instance,
    adapter: wgpu::Adapter,
    device: wgpu::Device,
    queue: wgpu::Queue,
    failure: Arc<Mutex<FailureState>>,
}

impl DeviceOwner {
    pub(crate) fn create(policy: AdapterPolicy) -> Result<Self, DeviceError> {
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..Default::default()
        });
        let options = wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            force_fallback_adapter: matches!(policy, AdapterPolicy::Software),
            compatible_surface: None,
        };
        let adapter = pollster::block_on(instance.request_adapter(&options))
            .map_err(|_| DeviceError::AdapterUnavailable)?;
        let adapter_info = adapter.get_info();

        let class = classify_adapter(adapter_info.device_type)
            .ok_or(DeviceError::SoftwareAdapterRejected)?;
        let expected = if policy == AdapterPolicy::Hardware {
            AdapterClass::Hardware
        } else {
            AdapterClass::Software
        };
        if class != expected {
            return Err(DeviceError::SoftwareAdapterRejected);
        }

        validate_adapter_target_formats(&adapter).map_err(DeviceError::UnsupportedTargetFormat)?;

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
        Ok(Self {
            instance,
            adapter,
            device,
            queue,
            failure,
        })
    }

    pub(crate) fn poll_until_submission(
        &self,
        submission: wgpu::SubmissionIndex,
        timeout: Duration,
    ) -> Result<(), DeviceError> {
        if timeout.is_zero() {
            return Err(DeviceError::InvalidTimeout);
        }
        self.ensure_healthy()?;
        let result = self
            .device
            .poll(wgpu::PollType::Wait {
                submission_index: Some(submission),
                timeout: Some(timeout),
            })
            .map(|_| ())
            .map_err(map_poll_error);
        self.complete_operation(result)
    }

    pub(crate) fn adapter_info(&self) -> wgpu::AdapterInfo {
        self.adapter.get_info()
    }

    pub(crate) fn device(&self) -> &wgpu::Device {
        &self.device
    }

    pub(crate) fn queue(&self) -> &wgpu::Queue {
        &self.queue
    }

    /// Preserve the operation failure alongside any asynchronous device failure.
    pub(crate) fn complete_operation<T>(
        &self,
        result: Result<T, DeviceError>,
    ) -> Result<T, DeviceError> {
        reconcile_error_state(result, None, self.ensure_healthy().err())
    }

    pub(crate) fn with_error_scopes<T>(
        &self,
        operation: impl FnOnce(&wgpu::Device) -> Result<T, DeviceError>,
    ) -> Result<T, DeviceError> {
        self.ensure_healthy()?;
        self.device.push_error_scope(wgpu::ErrorFilter::OutOfMemory);
        self.device.push_error_scope(wgpu::ErrorFilter::Validation);

        let operation_result = operation(&self.device);
        let validation_error = pollster::block_on(self.device.pop_error_scope());
        let allocation_error = pollster::block_on(self.device.pop_error_scope());

        let scoped_error = ScopedGpuDiagnostics::from_errors(validation_error, allocation_error);
        reconcile_error_state(operation_result, scoped_error, self.ensure_healthy().err())
    }

    /// Return the first asynchronous device failure observed by this owner.
    pub(crate) fn ensure_healthy(&self) -> Result<(), DeviceError> {
        let state = self
            .failure
            .lock()
            .map_err(|_| DeviceError::FailureStatePoisoned)?;
        state.error.clone().map_or(Ok(()), Err)
    }
}

fn map_poll_error(error: wgpu::PollError) -> DeviceError {
    match error {
        wgpu::PollError::Timeout => DeviceError::SubmissionTimeout,
        wgpu::PollError::WrongSubmissionIndex(requested, last) => {
            DeviceError::WrongSubmissionIndex(requested, last)
        }
    }
}

fn install_failure_callbacks(device: &wgpu::Device, failure: &Arc<Mutex<FailureState>>) {
    let lost_failure = Arc::clone(failure);
    device.set_device_lost_callback(move |reason, message| {
        record_failure(
            &lost_failure,
            DeviceError::DeviceLost(format!("{reason:?}: {message}")),
        );
    });

    let error_failure = Arc::clone(failure);
    device.on_uncaptured_error(Arc::new(move |error| {
        record_failure(
            &error_failure,
            DeviceError::UncapturedGpuError(error.to_string()),
        );
    }));
}

fn record_failure(failure: &Arc<Mutex<FailureState>>, error: DeviceError) {
    match failure.lock() {
        Ok(mut state) => {
            state.error.get_or_insert(error);
        }
        Err(poisoned) => {
            let mut state = poisoned.into_inner();
            state.error.get_or_insert(DeviceError::FailureStatePoisoned);
        }
    }
}

impl Drop for DeviceOwner {
    fn drop(&mut self) {
        self.device.destroy();
        self.instance.poll_all(true);
    }
}

#[cfg(all(test, feature = "infra-render-001"))]
#[path = "error_scope_tests.rs"]
mod error_scope_tests;
