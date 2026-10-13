use std::sync::Arc;

use engine_platform::NativeWindow;
use engine_render_api::{AdapterPolicy, RenderFrame, TargetSize};
use engine_render_wgpu::WgpuSurfaceRenderer;

use super::error::LiveSceneError;

/// Owns the native window surface and the renderer attached to it.
pub(super) struct LiveSurface {
    renderer: Option<WgpuSurfaceRenderer>,
    window: Arc<NativeWindow>,
    policy: AdapterPolicy,
    render_size: Option<TargetSize>,
    occluded: bool,
}

impl LiveSurface {
    pub(super) fn create(
        window: Arc<NativeWindow>,
        policy: AdapterPolicy,
    ) -> Result<Self, LiveSceneError> {
        let window_size = window.inner_size();
        let render_size = target_size(window_size.width, window_size.height)?;
        let renderer = match render_size {
            Some(size) => Some(create_renderer(&window, size, policy)?),
            None => None,
        };

        Ok(Self {
            renderer,
            window,
            policy,
            render_size,
            occluded: false,
        })
    }

    pub(super) fn resize(&mut self, width: u32, height: u32) -> Result<(), LiveSceneError> {
        let Some(size) = target_size(width, height)? else {
            self.render_size = None;
            return Ok(());
        };

        match self.renderer.as_mut() {
            Some(renderer) => renderer.resize(size).map_err(|error| {
                LiveSceneError::debug_context("resize live surface renderer", error)
            })?,
            None => {
                self.renderer = Some(create_renderer(&self.window, size, self.policy)?);
            }
        }

        self.render_size = Some(size);
        Ok(())
    }

    pub(super) fn set_occluded(&mut self, occluded: bool) {
        self.occluded = occluded;
    }

    pub(super) fn render_target(&self) -> Option<TargetSize> {
        if self.occluded || self.renderer.is_none() {
            return None;
        }

        self.render_size
    }

    pub(super) fn present(&mut self, frame: &RenderFrame<'_>) -> Result<(), LiveSceneError> {
        self.renderer
            .as_mut()
            .ok_or_else(|| LiveSceneError::new("live surface renderer is not initialized"))?
            .present(frame)
            .map_err(|error| LiveSceneError::debug_context("present live physics frame", error))
    }

    pub(super) fn request_redraw(&self) {
        self.window.request_redraw();
    }
}

fn create_renderer(
    window: &Arc<NativeWindow>,
    size: TargetSize,
    policy: AdapterPolicy,
) -> Result<WgpuSurfaceRenderer, LiveSceneError> {
    WgpuSurfaceRenderer::create_for_window(Arc::clone(window), size, policy)
        .map_err(|error| LiveSceneError::debug_context("create live surface renderer", error))
}

fn target_size(width: u32, height: u32) -> Result<Option<TargetSize>, LiveSceneError> {
    if width == 0 || height == 0 {
        return Ok(None);
    }

    TargetSize::try_new(width, height)
        .map(Some)
        .map_err(|error| {
            LiveSceneError::context("validate live surface size", format!("{error:?}"))
        })
}
