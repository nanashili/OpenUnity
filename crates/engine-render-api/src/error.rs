//! Typed failures shared by the renderer-neutral API and backend boundary.

/// Failures detected before or during a render submission.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RenderError {
    /// A target dimension is zero or exceeds the supported bound.
    InvalidTargetSize {
        /// Requested target width in pixels.
        width: u32,
        /// Requested target height in pixels.
        height: u32,
    },
    /// A target byte count overflowed a checked calculation.
    TargetAllocationOverflow,
    /// A referenced mesh index is outside the validated asset array.
    InvalidMeshReference {
        /// Zero-based mesh-array index supplied by the draw.
        index: u32,
    },
    /// A referenced material index is outside the validated asset array.
    InvalidMaterialReference {
        /// Zero-based material-array index supplied by the draw.
        index: u32,
    },
    /// A frame contains more draws than the bounded contract permits.
    DrawLimitExceeded {
        /// Number of draws observed when the limit was exceeded.
        count: usize,
    },
    /// A backend already owns the one permitted in-flight frame.
    FrameInFlight,
    /// Camera projection or affine-chain validation failed.
    InvalidCamera,
    /// Camera values or the camera affine chain are unsupported or unavailable.
    CameraUnavailable,
    /// A finite numeric contract was violated during frame preparation.
    Numeric,
    /// No adapter satisfies the explicit backend policy.
    AdapterUnavailable,
    /// The selected adapter lacks a required capability.
    UnsupportedCapabilities,
    /// Device creation was rejected.
    DeviceRequest,
    /// Resource allocation failed.
    Allocation,
    /// Shader or pipeline validation failed.
    Pipeline,
    /// The device was lost and this backend can no longer render.
    DeviceLost,
    /// Submission or readback did not complete before its bound.
    SubmissionTimeout,
    /// Mapping or readback failed.
    Readback,
    /// The published image size differs from its backend receipt size.
    ReceiptMismatch,
}

impl RenderError {
    /// Return whether the failure permanently invalidates a renderer instance.
    pub const fn is_device_failure(self) -> bool {
        matches!(
            self,
            Self::AdapterUnavailable
                | Self::UnsupportedCapabilities
                | Self::DeviceRequest
                | Self::Allocation
                | Self::Pipeline
                | Self::DeviceLost
                | Self::SubmissionTimeout
                | Self::Readback
        )
    }
}
