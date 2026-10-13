//! Live WGPU rendering directly into a native window surface.

use engine_render_api::{AdapterPolicy, RenderError, RenderFrame, TargetSize};

use crate::backend_mapping::{device_policy, map_device_error};
use crate::device::{DeviceOwner, create_for_window};
use crate::draw::{DrawPreparer, PreparedDraw, ValidatedDraws};
use crate::pipeline::OpaqueUnlitPipeline;
use crate::submission::{SurfaceSubmissionContext, submit_surface_frame};
use crate::surface_state::SurfaceState;
use crate::targets::DepthTarget;

/// A WGPU renderer that presents each frame directly to an owned native window.
pub struct WgpuSurfaceRenderer {
    // Drop the surface before its instance-owning device.
    surface: SurfaceState,
    device: DeviceOwner,
    pipeline: OpaqueUnlitPipeline,
    draw_preparer: DrawPreparer,
    depth_target: DepthTarget,
}

impl WgpuSurfaceRenderer {
    /// Create a surface-compatible renderer for an owned safe window handle.
    pub fn create_for_window<W>(
        window: W,
        initial_size: TargetSize,
        policy: AdapterPolicy,
    ) -> Result<Self, RenderError>
    where
        W: wgpu::WindowHandle + 'static,
    {
        let (device, native_surface) =
            create_for_window(window, device_policy(policy)).map_err(map_device_error)?;
        let surface = SurfaceState::create(native_surface, &device, initial_size)?;
        let pipeline = OpaqueUnlitPipeline::create_for_format(&device, surface.format())
            .map_err(map_device_error)?;
        let depth_target = create_depth_target(&device, initial_size)?;

        Ok(Self {
            surface,
            device,
            pipeline,
            draw_preparer: DrawPreparer::new(),
            depth_target,
        })
    }

    /// Reconfigure the surface and replace its matching depth target.
    pub fn resize(&mut self, size: TargetSize) -> Result<(), RenderError> {
        if self.surface.size() == size {
            return Ok(());
        }

        let depth_target = create_depth_target(&self.device, size)?;
        self.surface.resize(&self.device, size)?;
        self.depth_target = depth_target;
        Ok(())
    }

    /// Render and present a frame without copying pixels back to the CPU.
    pub fn present(&mut self, frame: &RenderFrame<'_>) -> Result<(), RenderError> {
        self.validate_surface_size(frame.target_size())?;
        self.device.ensure_healthy().map_err(map_device_error)?;

        let validated_draws = self.validate_frame(frame)?;
        let prepared_draws = self.prepare_draws(frame, validated_draws)?;
        let submission_draws = prepared_draws
            .iter()
            .map(PreparedDraw::as_submission_draw)
            .collect::<Vec<_>>();

        let surface_texture = self.surface.acquire(&self.device)?;
        let suboptimal = surface_texture.suboptimal;
        let color_view = surface_texture
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let submission_context = SurfaceSubmissionContext::new(
            self.device.device(),
            self.device.queue(),
            &color_view,
            &self.depth_target.view,
            self.pipeline.render_pipeline(),
            clear_color(frame.clear_rgba8()),
        );
        let draw_count = self
            .device
            .with_error_scopes(|_| {
                submit_surface_frame(&submission_context, &submission_draws).map_err(|error| {
                    crate::device::DeviceError::ScopedGpuError(format!("{error:?}"))
                })
            })
            .map_err(map_device_error)?;

        if draw_count != validated_draws.enabled_count() {
            return Err(RenderError::Allocation);
        }

        surface_texture.present();
        self.surface.note_suboptimal_frame(suboptimal);
        self.device.ensure_healthy().map_err(map_device_error)
    }

    fn validate_surface_size(&self, frame_size: TargetSize) -> Result<(), RenderError> {
        let surface_size = self.surface.size();
        if frame_size == surface_size {
            return Ok(());
        }

        Err(RenderError::SurfaceSizeMismatch {
            frame_width: frame_size.width(),
            frame_height: frame_size.height(),
            surface_width: surface_size.width(),
            surface_height: surface_size.height(),
        })
    }

    fn validate_frame(&self, frame: &RenderFrame<'_>) -> Result<ValidatedDraws, RenderError> {
        self.draw_preparer.prevalidate(
            frame.camera(),
            frame.target_size(),
            frame.meshes(),
            frame.materials(),
            frame.draws(),
        )
    }

    fn prepare_draws(
        &self,
        frame: &RenderFrame<'_>,
        validated_draws: ValidatedDraws,
    ) -> Result<Vec<PreparedDraw>, RenderError> {
        let mut prepared_draws = Vec::with_capacity(validated_draws.enabled_count());
        for draw in frame.draws().filter(|draw| draw.enabled()) {
            prepared_draws.push(self.draw_preparer.prepare(
                &self.device,
                frame.camera(),
                frame.target_size(),
                frame.meshes(),
                frame.materials(),
                draw,
            )?);
        }

        if prepared_draws.len() != validated_draws.enabled_count() {
            return Err(RenderError::Allocation);
        }
        Ok(prepared_draws)
    }
}

fn create_depth_target(device: &DeviceOwner, size: TargetSize) -> Result<DepthTarget, RenderError> {
    device
        .with_error_scopes(|gpu| Ok(DepthTarget::create(gpu, size)))
        .map_err(map_device_error)
}

fn clear_color(rgba8: [u8; 4]) -> wgpu::Color {
    wgpu::Color {
        r: f64::from(rgba8[0]) / 255.0,
        g: f64::from(rgba8[1]) / 255.0,
        b: f64::from(rgba8[2]) / 255.0,
        a: 1.0,
    }
}
