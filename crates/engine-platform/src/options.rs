/// Initial title and physical size for a native window.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WindowOptions {
    /// Text displayed in the native window title bar.
    pub title: String,
    /// Requested window width in physical pixels.
    pub width: u32,
    /// Requested window height in physical pixels.
    pub height: u32,
}

impl WindowOptions {
    /// Creates window options. Zero dimensions are rejected by `run_windowed`.
    pub fn new(title: impl Into<String>, width: u32, height: u32) -> Self {
        Self {
            title: title.into(),
            width,
            height,
        }
    }
}
