//! Swappable synchronous backend seam.

use crate::{RenderError, RenderFrame, RenderedImage};

/// Renderer-neutral synchronous backend contract.
pub trait RenderBackend {
    /// Render one validated frame and return checked readback bytes.
    fn render(&mut self, frame: &RenderFrame<'_>) -> Result<RenderedImage, RenderError>;
}
