//! Independent panel layout and versioned JSON; no Unity layout compatibility.
//!
//! Sizes use logical UI points. The inclusive 100–800 point bounds and default
//! sizes are independent OpenUnity authoring choices. Persistence callers must
//! enforce the protected-root and owned-file write policy before saving bytes.

use std::fmt;

use serde::{Deserialize, Serialize};

/// Minimum panel dimension in logical UI points.
pub const MIN_PANEL_SIZE: f32 = 100.0;
/// Maximum panel dimension in logical UI points.
pub const MAX_PANEL_SIZE: f32 = 800.0;
/// Maximum accepted layout JSON size, checked before parsing.
pub const MAX_LAYOUT_BYTES: usize = 64 * 1024;

/// Independent panel visibility and dimensions.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Layout {
    /// Whether the project browser is visible.
    pub project: bool,
    /// Whether the hierarchy is visible.
    pub hierarchy: bool,
    /// Whether the inspector is visible.
    pub inspector: bool,
    /// Whether the console is visible.
    pub console: bool,
    /// Left panel width in logical UI points.
    pub left_width: f32,
    /// Right panel width in logical UI points.
    pub right_width: f32,
    /// Console height in logical UI points.
    pub console_height: f32,
}

impl Default for Layout {
    fn default() -> Self {
        Self {
            project: true,
            hierarchy: true,
            inspector: true,
            console: true,
            left_width: 240.0,
            right_width: 280.0,
            console_height: 180.0,
        }
    }
}

/// Failure to validate, encode or decode an independent layout.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum LayoutError {
    /// A named dimension is nonfinite or outside the inclusive bounds.
    InvalidSize(&'static str),
    /// Input exceeds [`MAX_LAYOUT_BYTES`].
    InputTooLarge,
    /// Input belongs to another format.
    UnsupportedFormat,
    /// Input uses an unsupported layout version.
    UnsupportedVersion(u32),
    /// JSON is malformed, contains duplicate/unknown fields, or has wrong types.
    Json(String),
}

impl fmt::Display for LayoutError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidSize(field) => write!(
                formatter,
                "layout validation failed: {field} must be finite and within 100..=800 points"
            ),
            Self::InputTooLarge => {
                formatter.write_str("layout decode failed: input exceeds 65536 bytes")
            }
            Self::UnsupportedFormat => {
                formatter.write_str("layout decode failed: expected openunity.layout format")
            }
            Self::UnsupportedVersion(version) => write!(
                formatter,
                "layout decode failed: unsupported version {version}"
            ),
            Self::Json(detail) => write!(formatter, "layout JSON operation failed: {detail}"),
        }
    }
}

impl std::error::Error for LayoutError {}

// Keep every wire field explicit: serde's struct visitor rejects duplicate keys,
// and deny_unknown_fields rejects fields outside this version's contract.
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct LayoutFile {
    format: String,
    version: u32,
    project: bool,
    hierarchy: bool,
    inspector: bool,
    console: bool,
    left_width: f32,
    right_width: f32,
    console_height: f32,
}

impl Layout {
    /// Restores all visibility flags and dimensions to [`Layout::default`].
    pub fn reset(&mut self) {
        *self = Self::default();
    }

    /// Checks every dimension, including dimensions of hidden panels.
    pub fn validate(&self) -> Result<(), LayoutError> {
        for (field, value) in [
            ("left_width", self.left_width),
            ("right_width", self.right_width),
            ("console_height", self.console_height),
        ] {
            if !value.is_finite() || !(MIN_PANEL_SIZE..=MAX_PANEL_SIZE).contains(&value) {
                return Err(LayoutError::InvalidSize(field));
            }
        }
        Ok(())
    }

    /// Validates and encodes `openunity.layout` version 1 JSON.
    ///
    /// This performs no filesystem writes; callers enforce safe save locations.
    pub fn encode(&self) -> Result<Vec<u8>, LayoutError> {
        self.validate()?;
        let file = LayoutFile {
            format: "openunity.layout".into(),
            version: 1,
            project: self.project,
            hierarchy: self.hierarchy,
            inspector: self.inspector,
            console: self.console,
            left_width: self.left_width,
            right_width: self.right_width,
            console_height: self.console_height,
        };
        serde_json::to_vec_pretty(&file).map_err(|error| LayoutError::Json(error.to_string()))
    }

    /// Decodes and validates bounded `openunity.layout` version 1 JSON.
    ///
    /// Unknown, duplicate and missing fields are errors. Failure returns no
    /// partial layout and cannot modify an existing layout.
    pub fn decode(bytes: &[u8]) -> Result<Self, LayoutError> {
        if bytes.len() > MAX_LAYOUT_BYTES {
            return Err(LayoutError::InputTooLarge);
        }
        let file: LayoutFile =
            serde_json::from_slice(bytes).map_err(|error| LayoutError::Json(error.to_string()))?;
        if file.format != "openunity.layout" {
            return Err(LayoutError::UnsupportedFormat);
        }
        if file.version != 1 {
            return Err(LayoutError::UnsupportedVersion(file.version));
        }
        let layout = Self {
            project: file.project,
            hierarchy: file.hierarchy,
            inspector: file.inspector,
            console: file.console,
            left_width: file.left_width,
            right_width: file.right_width,
            console_height: file.console_height,
        };
        layout.validate()?;
        Ok(layout)
    }
}
