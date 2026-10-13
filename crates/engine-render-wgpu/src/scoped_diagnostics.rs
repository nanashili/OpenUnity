//! Typed diagnostics collected from WGPU validation and OOM error scopes.

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ScopedGpuDiagnostics {
    validation: Option<String>,
    allocation: Option<String>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ScopedGpuClass {
    Validation,
    Allocation,
    ValidationAndAllocation,
}

impl ScopedGpuDiagnostics {
    pub(crate) fn from_errors(
        validation: Option<wgpu::Error>,
        allocation: Option<wgpu::Error>,
    ) -> Option<Self> {
        if validation.is_none() && allocation.is_none() {
            return None;
        }

        Some(Self {
            validation: validation.map(|error| format!("{error:?}")),
            allocation: allocation.map(|error| format!("{error:?}")),
        })
    }

    pub(crate) fn class(&self) -> ScopedGpuClass {
        match (self.validation.is_some(), self.allocation.is_some()) {
            (true, true) => ScopedGpuClass::ValidationAndAllocation,
            (true, false) => ScopedGpuClass::Validation,
            (false, true) => ScopedGpuClass::Allocation,
            (false, false) => unreachable!("empty scoped GPU diagnostics"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{ScopedGpuClass, ScopedGpuDiagnostics};
    use crate::backend_mapping::map_device_error;
    use crate::device::DeviceError;
    use engine_render_api::RenderError;

    #[test]
    fn dependency_errors_preserve_diagnostics_and_public_classification() {
        for (validation, allocation, class, public_error) in [
            (
                true,
                false,
                ScopedGpuClass::Validation,
                RenderError::Pipeline,
            ),
            (
                false,
                true,
                ScopedGpuClass::Allocation,
                RenderError::Allocation,
            ),
            (
                true,
                true,
                ScopedGpuClass::ValidationAndAllocation,
                RenderError::Pipeline,
            ),
        ] {
            let validation_error = validation.then(|| wgpu::Error::Validation {
                source: Box::new(std::io::Error::other("validation source")),
                description: "invalid resource".into(),
            });
            let allocation_error = allocation.then(|| wgpu::Error::OutOfMemory {
                source: Box::new(std::io::Error::other("allocation source")),
            });
            let diagnostics =
                ScopedGpuDiagnostics::from_errors(validation_error, allocation_error).unwrap();

            assert_eq!(diagnostics.class(), class);
            if validation {
                assert!(
                    diagnostics
                        .validation
                        .as_ref()
                        .unwrap()
                        .contains("invalid resource")
                );
            }
            if allocation {
                assert!(
                    diagnostics
                        .allocation
                        .as_ref()
                        .unwrap()
                        .contains("allocation source")
                );
            }
            assert_eq!(
                map_device_error(DeviceError::ScopedGpuDiagnostics(diagnostics)),
                public_error
            );
        }
        assert!(ScopedGpuDiagnostics::from_errors(None, None).is_none());
    }
}
