//! Preference values, bounds and errors.

use std::fmt;
use std::path::PathBuf;

use crate::write_guard::WriteError;

/// Maximum preference document size accepted before JSON parsing.
pub const MAX_PREFERENCES_BYTES: usize = 64 * 1024;
/// Largest accepted positive snapping increment.
pub const MAX_SNAP_STEP: f32 = 1000.0;
/// Largest accepted undo/redo history capacity.
pub const MAX_HISTORY_CAPACITY: usize = 1024;

/// Independent OpenUnity editor settings, unrelated to Unity project settings.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Preferences {
    /// Whether the authoring viewport grid is visible.
    pub grid_visible: bool,
    /// Finite, positive snapping increment, at most [`MAX_SNAP_STEP`].
    pub snap_step: f32,
    /// Maximum retained undo/redo snapshots, including zero to disable retention.
    pub history_capacity: usize,
}

impl Default for Preferences {
    fn default() -> Self {
        Self {
            grid_visible: true,
            snap_step: 0.5,
            history_capacity: 100,
        }
    }
}

/// A validation, serialization, or guarded filesystem failure.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PreferencesError {
    /// The snapping increment is nonfinite, nonpositive, or above its limit.
    InvalidSnapStep,
    /// History capacity exceeds [`MAX_HISTORY_CAPACITY`].
    InvalidHistoryCapacity,
    /// Input exceeds [`MAX_PREFERENCES_BYTES`].
    InputTooLarge,
    /// The input format marker is not `openunity.preferences`.
    UnsupportedFormat,
    /// The input version is unsupported.
    UnsupportedVersion(u32),
    /// Malformed JSON, including missing, duplicate, unknown, or mistyped fields.
    Json(String),
    /// The preference file could not be read.
    Io {
        /// Path whose read failed.
        path: PathBuf,
        /// Human-readable filesystem error.
        message: String,
    },
    /// A path or write was rejected by the shared document guard.
    Write(WriteError),
}

impl fmt::Display for PreferencesError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidSnapStep => formatter
                .write_str("preferences snap_step must be finite, positive, and at most 1000"),
            Self::InvalidHistoryCapacity => {
                formatter.write_str("preferences history_capacity must be at most 1024")
            }
            Self::InputTooLarge => formatter.write_str("preferences input exceeds 65536 bytes"),
            Self::UnsupportedFormat => formatter.write_str("expected openunity.preferences format"),
            Self::UnsupportedVersion(version) => {
                write!(formatter, "unsupported preferences version {version}")
            }
            Self::Json(message) => {
                write!(formatter, "preferences JSON operation failed: {message}")
            }
            Self::Io { path, message } => write!(formatter, "{}: {message}", path.display()),
            Self::Write(error) => error.fmt(formatter),
        }
    }
}

impl std::error::Error for PreferencesError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Write(error) => Some(error),
            _ => None,
        }
    }
}

impl From<WriteError> for PreferencesError {
    fn from(error: WriteError) -> Self {
        Self::Write(error)
    }
}

impl Preferences {
    /// Checks snapping and history bounds without changing any setting.
    pub fn validate(&self) -> Result<(), PreferencesError> {
        if !self.snap_step.is_finite() || self.snap_step <= 0.0 || self.snap_step > MAX_SNAP_STEP {
            return Err(PreferencesError::InvalidSnapStep);
        }
        if self.history_capacity > MAX_HISTORY_CAPACITY {
            return Err(PreferencesError::InvalidHistoryCapacity);
        }
        Ok(())
    }
}
