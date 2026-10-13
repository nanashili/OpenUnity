//! Ordered one-draw command submission against shared frame targets.

use crate::readback::ReadbackBuffer;
use crate::targets::FrameTargets;

/// GPU buffers prepared by the draw owner and retained through completion.
pub(crate) struct PreparedDraw<'a> {
    pub(crate) vertex_buffer: &'a wgpu::Buffer,
    pub(crate) index_buffer: &'a wgpu::Buffer,
    pub(crate) index_count: u32,
}

impl PreparedDraw<'_> {
    pub(crate) fn validate(&self) -> Result<(), SubmissionError> {
        if self.index_count == 0 || !self.index_count.is_multiple_of(3) {
            return Err(SubmissionError::InvalidIndexCount(self.index_count));
        }
        Ok(())
    }
}

/// Whether this step begins the frame or continues an earlier draw sequence.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum AttachmentLoad {
    Clear,
    Load,
}

pub(crate) struct FrameSubmissionContext<'frame> {
    device: &'frame wgpu::Device,
    queue: &'frame wgpu::Queue,
    targets: &'frame FrameTargets,
    pipeline: &'frame wgpu::RenderPipeline,
    clear: wgpu::Color,
}

impl<'frame> FrameSubmissionContext<'frame> {
    pub(crate) fn new(
        device: &'frame wgpu::Device,
        queue: &'frame wgpu::Queue,
        targets: &'frame FrameTargets,
        pipeline: &'frame wgpu::RenderPipeline,
        clear: wgpu::Color,
    ) -> Self {
        Self {
            device,
            queue,
            targets,
            pipeline,
            clear,
        }
    }
}

pub(crate) struct FrameSubmission<'draw> {
    pub(crate) index: wgpu::SubmissionIndex,
    pub(crate) draw_count: usize,
    pub(crate) copied_readback: bool,
    _retained_vertex_buffer: Option<&'draw wgpu::Buffer>,
    _retained_index_buffer: Option<&'draw wgpu::Buffer>,
}

/// Submit one ordered draw, or an empty clear step, against shared targets.
///
/// The caller retains the `PreparedDraw` owner until the returned submission has
/// completed. Set `load` to `Clear` exactly once, then use `Load`; set `readback`
/// only on the final step. Queue ordering makes each step observe earlier draws.
pub(crate) fn submit_indexed_step<'draw>(
    context: &FrameSubmissionContext<'_>,
    readback: Option<&ReadbackBuffer>,
    load: AttachmentLoad,
    draw: Option<&'draw PreparedDraw<'_>>,
) -> Result<FrameSubmission<'draw>, SubmissionError> {
    if let Some(draw) = draw {
        draw.validate()?;
    }
    if readback.is_none() && draw.is_none() {
        return Err(SubmissionError::EmptyStepWithoutReadback);
    }

    let mut encoder = context
        .device
        .create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("openunity-frame-step"),
        });
    {
        let color_attachment = wgpu::RenderPassColorAttachment {
            view: &context.targets.color_view,
            depth_slice: None,
            resolve_target: None,
            ops: wgpu::Operations {
                load: color_load(load, context.clear),
                store: wgpu::StoreOp::Store,
            },
        };
        let depth_attachment = wgpu::RenderPassDepthStencilAttachment {
            view: &context.targets.depth_view,
            depth_ops: Some(wgpu::Operations {
                load: depth_load(load),
                store: wgpu::StoreOp::Store,
            }),
            stencil_ops: None,
        };
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("openunity-indexed-draw-step"),
            color_attachments: &[Some(color_attachment)],
            depth_stencil_attachment: Some(depth_attachment),
            occlusion_query_set: None,
            timestamp_writes: None,
        });
        pass.set_pipeline(context.pipeline);
        if let Some(draw) = draw {
            pass.set_vertex_buffer(0, draw.vertex_buffer.slice(..));
            pass.set_index_buffer(draw.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
            pass.draw_indexed(0..draw.index_count, 0, 0..1);
        }
    }

    if let Some(readback) = readback {
        readback.encode_copy(&mut encoder, context.targets);
    }
    let index = context.queue.submit(std::iter::once(encoder.finish()));
    Ok(FrameSubmission {
        index,
        draw_count: if draw.is_some() { 1 } else { 0 },
        copied_readback: readback.is_some(),
        _retained_vertex_buffer: draw.map(|value| value.vertex_buffer),
        _retained_index_buffer: draw.map(|value| value.index_buffer),
    })
}

fn color_load(load: AttachmentLoad, clear: wgpu::Color) -> wgpu::LoadOp<wgpu::Color> {
    match load {
        AttachmentLoad::Clear => wgpu::LoadOp::Clear(clear),
        AttachmentLoad::Load => wgpu::LoadOp::Load,
    }
}

fn depth_load(load: AttachmentLoad) -> wgpu::LoadOp<f32> {
    match load {
        AttachmentLoad::Clear => wgpu::LoadOp::Clear(1.0),
        AttachmentLoad::Load => wgpu::LoadOp::Load,
    }
}

#[derive(Debug, Eq, PartialEq)]
pub(crate) enum SubmissionError {
    InvalidIndexCount(u32),
    EmptyStepWithoutReadback,
}
