//! WGPU state for the bounded opaque unlit triangle path.
//!
//! The production owner is `crates/engine-render-wgpu/src/pipeline.rs`.
//! Camera-space inversion and orthographic projection belong to the renderer
//! API/frame preparation layer. This backend receives finite clip-space
//! positions, validates them again at the submission boundary, and leaves
//! clipping to the fixed-function rasterizer.

use std::num::NonZeroU32;

use crate::device::{DeviceError, DeviceOwner};

/// The color target required by INFRA-RENDER-001.
const COLOR_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;
/// The depth target required by INFRA-RENDER-001.
const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;
/// One prepared vertex submitted to the opaque unlit pipeline.
///
/// `clip_position` already contains the contract's orthographic result:
/// `x/(half_height*aspect)`, `y/half_height`, `(z-near)/(far-near)`, `1`.
/// The fixed-function clip volume therefore uses x/y `[-w,w]` and z `[0,w]`.
#[repr(C)]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub(crate) struct PreparedVertex {
    /// Homogeneous position in hardware clip coordinates.
    pub(crate) clip_position: [f32; 4],
    /// Raw opaque RGBA bytes, consumed as four unsigned vertex components.
    pub(crate) rgba8: [u8; 4],
}

/// Errors raised immediately before a vertex buffer is uploaded.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum VertexPreparationError {
    /// A transformed vertex contained a NaN or an infinity.
    NonFinite {
        vertex_index: usize,
        component: usize,
    },
}

/// Reject non-finite transformed vertices before any GPU submission.
pub(crate) fn validate_prepared_vertices(
    vertices: &[PreparedVertex],
) -> Result<(), VertexPreparationError> {
    for (vertex_index, vertex) in vertices.iter().enumerate() {
        for (component, value) in vertex.clip_position.iter().copied().enumerate() {
            if !value.is_finite() {
                return Err(VertexPreparationError::NonFinite {
                    vertex_index,
                    component,
                });
            }
        }
    }

    Ok(())
}

/// The fixed pipeline state for an opaque, double-sided triangle list.
pub(crate) struct OpaqueUnlitPipeline {
    render_pipeline: wgpu::RenderPipeline,
}

impl OpaqueUnlitPipeline {
    /// Build the unlit pipeline through the device owner's error boundary.
    pub(crate) fn create(device: &DeviceOwner) -> Result<Self, DeviceError> {
        device.with_error_scopes(|gpu| Ok(Self::create_gpu_objects(gpu)))
    }

    fn create_gpu_objects(device: &wgpu::Device) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("openunity opaque unlit shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("unlit.wgsl").into()),
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("openunity opaque unlit pipeline layout"),
            bind_group_layouts: &[],
            push_constant_ranges: &[],
        });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("openunity opaque unlit pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vertex_main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &[vertex_layout()],
            },
            primitive: primitive_state(),
            depth_stencil: Some(depth_state()),
            multisample: multisample_state(),
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fragment_main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: COLOR_FORMAT,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            cache: None,
            multiview: None,
        });

        Self { render_pipeline }
    }

    /// Borrow the immutable pipeline for a render pass.
    pub(crate) fn render_pipeline(&self) -> &wgpu::RenderPipeline {
        &self.render_pipeline
    }
}

fn vertex_layout() -> wgpu::VertexBufferLayout<'static> {
    const ATTRIBUTES: [wgpu::VertexAttribute; 2] = [
        wgpu::VertexAttribute {
            format: wgpu::VertexFormat::Float32x4,
            offset: 0,
            shader_location: 0,
        },
        wgpu::VertexAttribute {
            format: wgpu::VertexFormat::Uint8x4,
            offset: 16,
            shader_location: 1,
        },
    ];

    wgpu::VertexBufferLayout {
        array_stride: std::mem::size_of::<PreparedVertex>() as u64,
        step_mode: wgpu::VertexStepMode::Vertex,
        attributes: &ATTRIBUTES,
    }
}

fn primitive_state() -> wgpu::PrimitiveState {
    wgpu::PrimitiveState {
        topology: wgpu::PrimitiveTopology::TriangleList,
        strip_index_format: None,
        front_face: wgpu::FrontFace::Ccw,
        cull_mode: None,
        unclipped_depth: false,
        polygon_mode: wgpu::PolygonMode::Fill,
        conservative: false,
    }
}

fn depth_state() -> wgpu::DepthStencilState {
    wgpu::DepthStencilState {
        format: DEPTH_FORMAT,
        depth_write_enabled: true,
        depth_compare: wgpu::CompareFunction::LessEqual,
        stencil: wgpu::StencilState::default(),
        bias: wgpu::DepthBiasState::default(),
    }
}

fn multisample_state() -> wgpu::MultisampleState {
    wgpu::MultisampleState {
        count: NonZeroU32::new(1).expect("one is nonzero").get(),
        mask: !0,
        alpha_to_coverage_enabled: false,
    }
}
