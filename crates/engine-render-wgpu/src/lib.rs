//! Standalone offscreen WGPU rendering backend.

mod backend_mapping;
mod device;
mod draw;
mod frame_execution;
mod pipeline;
mod readback;
mod renderer;
mod submission;
mod surface_renderer;
mod surface_state;
mod targets;

pub use renderer::WgpuRenderer;
pub use surface_renderer::WgpuSurfaceRenderer;
