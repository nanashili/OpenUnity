//! Standalone offscreen WGPU rendering backend.

mod backend_mapping;
mod device;
mod draw;
mod frame_execution;
mod pipeline;
mod readback;
mod renderer;
mod submission;
mod targets;

pub use renderer::WgpuRenderer;
