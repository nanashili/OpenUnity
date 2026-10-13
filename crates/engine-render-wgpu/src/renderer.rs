//! Frame sequencing for the standalone WGPU backend.

use std::time::Duration;

use engine_render_api::{AdapterPolicy, RenderBackend, RenderError, RenderFrame, RenderedImage};

use crate::backend_mapping::{device_policy, map_device_error};
use crate::device::DeviceOwner;
use crate::draw::DrawPreparer;
use crate::pipeline::OpaqueUnlitPipeline;

pub(crate) const GPU_WAIT: Duration = Duration::from_secs(5);

/// A synchronous offscreen renderer with one frame in flight at a time.
pub struct WgpuRenderer {
    pub(crate) requested_policy: AdapterPolicy,
    pub(crate) device: DeviceOwner,
    pub(crate) pipeline: OpaqueUnlitPipeline,
    pub(crate) draw_preparer: DrawPreparer,
    failed: Option<RenderError>,
    frame_in_flight: bool,
}

impl WgpuRenderer {
    /// Create a renderer using the explicitly selected adapter policy.
    pub fn create(policy: AdapterPolicy) -> Result<Self, RenderError> {
        let device = DeviceOwner::create(device_policy(policy)).map_err(map_device_error)?;
        let pipeline = OpaqueUnlitPipeline::create(&device)?;

        Ok(Self {
            requested_policy: policy,
            device,
            pipeline,
            draw_preparer: DrawPreparer::new(),
            failed: None,
            frame_in_flight: false,
        })
    }

    fn begin_frame(&mut self) -> Result<(), RenderError> {
        if let Some(error) = self.failed {
            return Err(error);
        }
        if self.frame_in_flight {
            return Err(RenderError::FrameInFlight);
        }
        if let Err(error) = self.device.ensure_healthy() {
            return Err(self.record_failure(map_device_error(error)));
        }

        self.frame_in_flight = true;
        Ok(())
    }

    fn record_failure(&mut self, error: RenderError) -> RenderError {
        if error.is_device_failure() {
            self.failed = Some(error);
        }
        error
    }
}

impl RenderBackend for WgpuRenderer {
    fn render(&mut self, frame: &RenderFrame<'_>) -> Result<RenderedImage, RenderError> {
        self.begin_frame()?;
        let result = self.render_in_flight(frame);
        self.frame_in_flight = false;

        result.map_err(|error| self.record_failure(error))
    }
}
