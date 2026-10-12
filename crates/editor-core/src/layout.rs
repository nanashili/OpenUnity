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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_preserves_visibility_and_boundary_sizes() {
        let layout = Layout {
            project: false,
            hierarchy: true,
            inspector: false,
            console: true,
            left_width: MIN_PANEL_SIZE,
            right_width: MAX_PANEL_SIZE,
            console_height: 321.5,
        };
        let encoded = layout.encode().unwrap();
        assert_eq!(Layout::decode(&encoded).unwrap(), layout);
        assert!(
            String::from_utf8(encoded)
                .unwrap()
                .contains("openunity.layout")
        );
    }

    #[test]
    fn reset_restores_every_field_even_from_an_invalid_layout() {
        let mut layout = Layout {
            project: false,
            hierarchy: false,
            inspector: false,
            console: false,
            left_width: f32::NAN,
            right_width: 1.0,
            console_height: 900.0,
        };
        layout.reset();
        assert_eq!(layout, Layout::default());
        assert!(layout.validate().is_ok());
    }

    #[test]
    fn all_dimensions_reject_nonfinite_and_out_of_bounds_values() {
        for invalid in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY, 99.0, 801.0] {
            for field in ["left_width", "right_width", "console_height"] {
                let mut layout = Layout::default();
                match field {
                    "left_width" => layout.left_width = invalid,
                    "right_width" => layout.right_width = invalid,
                    _ => layout.console_height = invalid,
                }
                assert_eq!(layout.validate(), Err(LayoutError::InvalidSize(field)));
                assert_eq!(layout.encode(), Err(LayoutError::InvalidSize(field)));
            }
        }
    }

    #[test]
    fn decode_rejects_invalid_sizes_and_unsupported_identity() {
        let json = String::from_utf8(Layout::default().encode().unwrap()).unwrap();
        assert_eq!(
            Layout::decode(json.replace("240.0", "99.0").as_bytes()),
            Err(LayoutError::InvalidSize("left_width"))
        );
        assert_eq!(
            Layout::decode(json.replace("\"version\": 1", "\"version\": 2").as_bytes()),
            Err(LayoutError::UnsupportedVersion(2))
        );
        assert_eq!(
            Layout::decode(
                json.replace("openunity.layout", "unrelated.layout")
                    .as_bytes()
            ),
            Err(LayoutError::UnsupportedFormat)
        );
    }

    #[test]
    fn decode_rejects_unknown_duplicate_and_missing_fields() {
        let json = String::from_utf8(Layout::default().encode().unwrap()).unwrap();
        for input in [
            json.replacen('{', "{\"extra\": true,", 1),
            json.replacen('{', "{\"version\": 1,", 1),
            json.replacen('{', "{\"left_width\": 200,", 1),
            json.replace("  \"project\": true,\n", ""),
        ] {
            assert!(matches!(
                Layout::decode(input.as_bytes()),
                Err(LayoutError::Json(_))
            ));
        }
    }

    #[test]
    fn decode_bounds_bytes_before_parsing() {
        let mut bytes = Layout::default().encode().unwrap();
        bytes.resize(MAX_LAYOUT_BYTES, b' ');
        assert_eq!(Layout::decode(&bytes).unwrap(), Layout::default());
        bytes.push(b' ');
        assert_eq!(Layout::decode(&bytes), Err(LayoutError::InputTooLarge));
    }
}
