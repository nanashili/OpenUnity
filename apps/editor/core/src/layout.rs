//! Independent panel layout and versioned JSON; no Unity layout compatibility.
//!
//! Sizes use logical UI points. The inclusive 100–800 point bounds and default
//! sizes are independent OpenUnity authoring choices. Persistence callers must
//! enforce the protected-root and owned-file write policy before saving bytes.

mod model;

pub use model::{Layout, LayoutError, MAX_LAYOUT_BYTES, MAX_PANEL_SIZE, MIN_PANEL_SIZE};

#[cfg(test)]
mod tests;
