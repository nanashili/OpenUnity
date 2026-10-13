use std::sync::Arc;

use raw_window_handle::{
    DisplayHandle, HandleError, HasDisplayHandle, HasWindowHandle, WindowHandle,
};
use winit::{
    dpi::PhysicalSize,
    window::{Window, WindowId},
};

/// Physical pixel dimensions reported by a native window.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct WindowSize {
    /// Width in physical pixels.
    pub width: u32,
    /// Height in physical pixels.
    pub height: u32,
}

/// An owned native window handle suitable for creating a renderer surface.
#[derive(Clone)]
pub struct NativeWindow {
    inner: Arc<Window>,
}

impl NativeWindow {
    pub(crate) fn new(inner: Arc<Window>) -> Self {
        Self { inner }
    }

    /// Returns the current framebuffer size in physical pixels.
    pub fn inner_size(&self) -> WindowSize {
        let size = self.inner.inner_size();
        WindowSize {
            width: size.width,
            height: size.height,
        }
    }

    /// Requests a redraw from the operating system.
    pub fn request_redraw(&self) {
        self.inner.request_redraw();
    }

    pub(crate) fn id(&self) -> WindowId {
        self.inner.id()
    }
}

impl HasWindowHandle for NativeWindow {
    fn window_handle(&self) -> Result<WindowHandle<'_>, HandleError> {
        self.inner.window_handle()
    }
}

impl HasDisplayHandle for NativeWindow {
    fn display_handle(&self) -> Result<DisplayHandle<'_>, HandleError> {
        self.inner.display_handle()
    }
}

impl From<PhysicalSize<u32>> for WindowSize {
    fn from(size: PhysicalSize<u32>) -> Self {
        Self {
            width: size.width,
            height: size.height,
        }
    }
}
