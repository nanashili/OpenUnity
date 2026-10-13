//! Renderer-neutral frame, camera, image, and backend contracts.

mod backend;
mod camera;
mod error;
mod frame;
mod image;

pub use backend::RenderBackend;
pub use camera::{OrthographicCamera, affine_from_snapshot_rows, transform_model_point};
pub use error::RenderError;
pub use frame::{DrawInstance, MAX_DRAW_INSTANCES, MAX_TARGET_DIMENSION, RenderFrame, TargetSize};
pub use image::{
    AdapterIdentity, AdapterPolicy, BackendCompletion, BackendReceipt, RenderedImage, TargetFormat,
};
