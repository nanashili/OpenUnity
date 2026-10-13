//! Real WGPU scope and callback failures at the resource-error boundary.

use super::{AdapterPolicy, DeviceError, DeviceOwner, ScopedGpuClass};
use crate::backend_mapping::map_device_error;
use engine_render_api::RenderError;

#[test]
fn resource_errors_preserve_primary_failures_when_validation_also_fails() {
    for (invalid_shader, callback_loss, operation_failure, expected) in [
        (true, false, true, RenderError::SubmissionTimeout),
        (true, true, true, RenderError::DeviceLost),
        (true, true, false, RenderError::DeviceLost),
        (false, true, true, RenderError::DeviceLost),
    ] {
        let owner = DeviceOwner::create(AdapterPolicy::Hardware)
            .expect("hardware adapter is required by rendering acceptance");
        let error = failed_operation(&owner, invalid_shader, callback_loss, operation_failure);

        assert_eq!(map_device_error(error.clone()), expected);
        assert_preserved_failures(error, invalid_shader, callback_loss, operation_failure);
    }
}

fn failed_operation(
    owner: &DeviceOwner,
    invalid_shader: bool,
    callback_loss: bool,
    operation_failure: bool,
) -> DeviceError {
    owner
        .with_error_scopes(|device| {
            if invalid_shader {
                let _invalid = device.create_shader_module(wgpu::ShaderModuleDescriptor {
                    label: Some("scope-validation-acceptance"),
                    source: wgpu::ShaderSource::Wgsl("invalid WGSL".into()),
                });
            }
            if callback_loss {
                device.destroy();
                device
                    .poll(wgpu::PollType::Wait {
                        submission_index: None,
                        timeout: Some(crate::renderer::GPU_WAIT),
                    })
                    .expect("bounded destroyed-device poll must dispatch its callback");
                assert!(matches!(
                    owner.ensure_healthy(),
                    Err(DeviceError::DeviceLost(_))
                ));
            }

            if operation_failure {
                Err(DeviceError::SubmissionTimeout)
            } else {
                Ok(())
            }
        })
        .expect_err("GPU validation or callback failure must reject success")
}

fn assert_preserved_failures(
    error: DeviceError,
    invalid_shader: bool,
    callback_loss: bool,
    operation_failure: bool,
) {
    match error {
        DeviceError::OperationAndScopedGpuError(primary, diagnostics) => {
            assert!(!callback_loss);
            assert!(matches!(*primary, DeviceError::SubmissionTimeout));
            assert_eq!(diagnostics.class(), ScopedGpuClass::Validation);
        }
        DeviceError::RecordedFailureAndScopedGpuError {
            recorded,
            operation,
            diagnostics,
        } => {
            assert!(callback_loss && invalid_shader);
            assert!(matches!(*recorded, DeviceError::DeviceLost(_)));
            assert_eq!(operation.is_some(), operation_failure);
            if let Some(primary) = operation {
                assert!(matches!(*primary, DeviceError::SubmissionTimeout));
            }
            assert_eq!(diagnostics.class(), ScopedGpuClass::Validation);
        }
        DeviceError::RecordedFailureAndOperation {
            recorded,
            operation,
        } => {
            assert!(callback_loss && !invalid_shader);
            assert!(matches!(*recorded, DeviceError::DeviceLost(_)));
            assert!(matches!(*operation, DeviceError::SubmissionTimeout));
        }
        other => panic!("expected preserved independent failures, got {other:?}"),
    }
}
