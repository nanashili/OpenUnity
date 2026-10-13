use std::error::Error;
use std::sync::Arc;

use crate::{NativeWindow, WindowAction, WindowEvent};

/// Receives the window lifecycle and input events for a native application.
pub trait WindowApplication {
    /// Error returned when application setup, event handling, or cleanup fails.
    type Error: Error + Send + Sync + 'static;

    /// Initializes or restores application resources for the active native window.
    fn resumed(&mut self, window: Arc<NativeWindow>) -> Result<(), Self::Error>;

    /// Handles one normalized platform event.
    fn event(&mut self, event: WindowEvent) -> Result<WindowAction, Self::Error>;

    /// Releases application resources when `run_windowed` finishes.
    ///
    /// Called once even when platform initialization or an application callback fails.
    fn exiting(&mut self) -> Result<(), Self::Error>;
}
