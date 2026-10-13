use std::time::Instant;

/// Platform events needed by a windowed application.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WindowEvent {
    /// The operating system suspended the windowed application.
    Suspended,
    /// The framebuffer size changed to the given physical pixel dimensions.
    Resized {
        /// Width in physical pixels.
        width: u32,
        /// Height in physical pixels.
        height: u32,
    },
    /// The OS requested a frame at the given monotonic time.
    RedrawRequested {
        /// Monotonic time at which the application should begin the frame.
        at: Instant,
    },
    /// The operating system requested that the application close.
    CloseRequested,
    /// The Escape key was pressed.
    EscapePressed,
    /// The Space key was pressed.
    SpacePressed,
    /// The window focus state changed.
    Focused(bool),
    /// The window occlusion state changed.
    Occluded(bool),
}

/// The application's decision after handling a window event.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WindowAction {
    /// Continue the native event loop.
    Continue,
    /// Exit the native event loop after this callback returns.
    Exit,
}
