use super::{DeviceError, ScopedGpuDiagnostics};

pub(super) fn reconcile_error_state<T>(
    operation_result: Result<T, DeviceError>,
    scoped_error: Option<ScopedGpuDiagnostics>,
    recorded_failure: Option<DeviceError>,
) -> Result<T, DeviceError> {
    match (operation_result, scoped_error, recorded_failure) {
        (operation, Some(diagnostics), Some(recorded)) => {
            Err(DeviceError::RecordedFailureAndScopedGpuError {
                recorded: Box::new(recorded),
                operation: operation.err().map(Box::new),
                diagnostics,
            })
        }
        (Err(operation), Some(scoped), None) => Err(DeviceError::OperationAndScopedGpuError(
            Box::new(operation),
            scoped,
        )),
        (Err(error), None, None) => Err(error),
        (Err(operation), None, Some(recorded)) => Err(DeviceError::RecordedFailureAndOperation {
            recorded: Box::new(recorded),
            operation: Box::new(operation),
        }),
        (Ok(_), Some(error), None) => Err(DeviceError::ScopedGpuDiagnostics(error)),
        (Ok(value), None, None) => Ok(value),
        (Ok(_), None, Some(recorded)) => Err(recorded),
    }
}
