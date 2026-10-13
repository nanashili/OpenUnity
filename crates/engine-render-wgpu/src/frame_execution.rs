//! Ordered frame execution phases for the WGPU backend.

use engine_render_api::{
    BackendCompletion, BackendReceipt, RenderError, RenderFrame, RenderedImage, TargetFormat,
    TargetSize,
};

use crate::backend_mapping::{adapter_identity, map_device_error, map_readback_error};
use crate::device::DeviceError;
use crate::draw::{PreparedDraw, ValidatedDraws};
use crate::readback::{ReadbackBuffer, map_rgba8};
use crate::renderer::WgpuRenderer;
use crate::submission::{AttachmentLoad, FrameSubmissionContext};
use crate::targets::FrameTargets;

use super::renderer::GPU_WAIT;

impl WgpuRenderer {
    pub(crate) fn render_in_flight(
        &mut self,
        frame: &RenderFrame<'_>,
    ) -> Result<RenderedImage, RenderError> {
        let (size, validated_draws) = self.validate_frame(frame)?;
        let (targets, readback) = self.allocate_frame_resources(size)?;
        let (submitted_draw_count, load) =
            self.submit_ordered_draws(frame, &targets, &validated_draws)?;
        let (rgba8, copied_readback) =
            self.complete_readback(frame, &targets, &readback, size, load)?;

        if submitted_draw_count != validated_draws.enabled_count() {
            return Err(RenderError::Allocation);
        }

        self.publish_image(
            frame,
            rgba8,
            submitted_draw_count,
            validated_draws,
            copied_readback,
        )
    }

    fn validate_frame(
        &self,
        frame: &RenderFrame<'_>,
    ) -> Result<(TargetSize, ValidatedDraws), RenderError> {
        let size = target_size(frame)?;
        let validated_draws = self.draw_preparer.prevalidate(
            frame.camera(),
            frame.target_size(),
            frame.meshes(),
            frame.materials(),
            frame.draws(),
        )?;
        Ok((size, validated_draws))
    }

    fn allocate_frame_resources(
        &self,
        size: TargetSize,
    ) -> Result<(FrameTargets, ReadbackBuffer), RenderError> {
        let targets = self
            .device
            .with_error_scopes(|device| {
                FrameTargets::create(device, size)
                    .map_err(|error| DeviceError::ScopedGpuError(format!("{error:?}")))
            })
            .map_err(map_device_error)?;
        let readback = self
            .device
            .with_error_scopes(|device| {
                ReadbackBuffer::create(device, size)
                    .map_err(|error| DeviceError::ScopedGpuError(format!("{error:?}")))
            })
            .map_err(map_device_error)?;
        Ok((targets, readback))
    }

    fn submit_ordered_draws(
        &self,
        frame: &RenderFrame<'_>,
        targets: &FrameTargets,
        validated_draws: &ValidatedDraws,
    ) -> Result<(usize, AttachmentLoad), RenderError> {
        let mut next_load = AttachmentLoad::Clear;
        let mut submitted_draw_count = 0usize;
        for draw in frame.draws().filter(|draw| draw.enabled()) {
            let prepared = self.draw_preparer.prepare(
                &self.device,
                frame.camera(),
                frame.target_size(),
                frame.meshes(),
                frame.materials(),
                draw,
            )?;
            submitted_draw_count = submitted_draw_count
                .checked_add(self.submit_draw_and_wait(frame, targets, &prepared, next_load)?)
                .ok_or(RenderError::Allocation)?;
            next_load = AttachmentLoad::Load;
        }
        if submitted_draw_count > validated_draws.enabled_count() {
            return Err(RenderError::Allocation);
        }
        Ok((submitted_draw_count, next_load))
    }

    fn complete_readback(
        &self,
        frame: &RenderFrame<'_>,
        targets: &FrameTargets,
        readback: &ReadbackBuffer,
        size: TargetSize,
        load: AttachmentLoad,
    ) -> Result<(Vec<u8>, bool), RenderError> {
        let (copy_submission, draw_count, copied_readback) =
            self.submit_readback(frame, targets, Some(readback), load)?;
        if draw_count != 0 || !copied_readback {
            return Err(RenderError::Readback);
        }
        self.wait_for_submission(copy_submission.clone())?;
        let readback_result = map_rgba8(
            self.device.device(),
            readback,
            copy_submission,
            size,
            GPU_WAIT,
        )
        .map_err(map_readback_error);
        let rgba8 = self
            .device
            .complete_operation(readback_result)
            .map_err(map_device_error)?;
        Ok((rgba8, copied_readback))
    }

    fn publish_image(
        &self,
        frame: &RenderFrame<'_>,
        rgba8: Vec<u8>,
        draw_count: usize,
        validated_draws: ValidatedDraws,
        readback_complete: bool,
    ) -> Result<RenderedImage, RenderError> {
        let receipt = BackendReceipt::new(
            self.requested_policy,
            adapter_identity(&self.device.adapter_info()),
            TargetFormat::Rgba8Unorm,
            frame,
            BackendCompletion::new(
                draw_count,
                validated_draws.triangle_count(),
                readback_complete,
            ),
        );

        self.device.ensure_healthy().map_err(map_device_error)?;

        RenderedImage::try_new(frame.target_size(), rgba8, receipt)
    }

    fn submit_draw_and_wait(
        &self,
        frame: &RenderFrame<'_>,
        targets: &FrameTargets,
        draw: &PreparedDraw,
        load: AttachmentLoad,
    ) -> Result<usize, RenderError> {
        let submission_draw = draw.as_submission_draw();
        let context = FrameSubmissionContext::new(
            self.device.device(),
            self.device.queue(),
            targets,
            self.pipeline.render_pipeline(),
            clear_color(frame.clear_rgba8()),
        );
        let submission = self
            .device
            .with_error_scopes(|_| {
                crate::submission::submit_indexed_step(&context, None, load, Some(&submission_draw))
                    .map_err(|error| DeviceError::ScopedGpuError(format!("{error:?}")))
            })
            .map_err(map_device_error)?;
        if submission.draw_count != 1 || submission.copied_readback {
            return Err(RenderError::Allocation);
        }
        self.wait_for_submission(submission.index)?;
        Ok(submission.draw_count)
    }

    fn submit_readback(
        &self,
        frame: &RenderFrame<'_>,
        targets: &FrameTargets,
        readback: Option<&ReadbackBuffer>,
        load: AttachmentLoad,
    ) -> Result<(wgpu::SubmissionIndex, usize, bool), RenderError> {
        let context = FrameSubmissionContext::new(
            self.device.device(),
            self.device.queue(),
            targets,
            self.pipeline.render_pipeline(),
            clear_color(frame.clear_rgba8()),
        );
        let submission = self
            .device
            .with_error_scopes(|_| {
                crate::submission::submit_indexed_step(&context, readback, load, None)
                    .map_err(|error| DeviceError::ScopedGpuError(format!("{error:?}")))
            })
            .map_err(map_device_error)?;
        Ok((
            submission.index,
            submission.draw_count,
            submission.copied_readback,
        ))
    }

    fn wait_for_submission(&self, index: wgpu::SubmissionIndex) -> Result<(), RenderError> {
        self.device
            .poll_until_submission(index, GPU_WAIT)
            .map_err(map_device_error)
    }
}

fn target_size(frame: &RenderFrame<'_>) -> Result<TargetSize, RenderError> {
    TargetSize::try_new(frame.target_size().width(), frame.target_size().height()).map_err(|_| {
        RenderError::InvalidTargetSize {
            width: frame.target_size().width(),
            height: frame.target_size().height(),
        }
    })
}

fn clear_color(rgba8: [u8; 4]) -> wgpu::Color {
    wgpu::Color {
        r: f64::from(rgba8[0]) / 255.0,
        g: f64::from(rgba8[1]) / 255.0,
        b: f64::from(rgba8[2]) / 255.0,
        a: 1.0,
    }
}

#[cfg(all(test, feature = "infra-render-001"))]
#[path = "frame_execution_tests.rs"]
mod frame_execution_tests;
