//! Native window lifecycle and platform-neutral window events.

mod application;
mod error;
mod event_loop;
mod events;
mod options;
mod window;

pub use application::WindowApplication;
pub use error::{PlatformFailure, WindowRunError};
pub use event_loop::run_windowed;
pub use events::{WindowAction, WindowEvent};
pub use options::WindowOptions;
pub use window::{NativeWindow, WindowSize};
