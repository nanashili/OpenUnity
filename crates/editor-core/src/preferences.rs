//! Validated, independently versioned editor preferences and guarded persistence.

use std::fmt;
use std::fs::{self, File};
use std::io::{self, Read};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::write_guard::{WriteError, WritePolicy};

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

// Explicit struct fields reject duplicates; deny_unknown_fields closes the
// supported version's wire shape without accepting arbitrary settings.
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct PreferencesFile {
    format: String,
    version: u32,
    grid_visible: bool,
    snap_step: f32,
    history_capacity: usize,
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

    /// Validates and encodes `openunity.preferences` version 1 JSON.
    pub fn encode(&self) -> Result<Vec<u8>, PreferencesError> {
        self.validate()?;
        serde_json::to_vec_pretty(&PreferencesFile {
            format: "openunity.preferences".into(),
            version: 1,
            grid_visible: self.grid_visible,
            snap_step: self.snap_step,
            history_capacity: self.history_capacity,
        })
        .map_err(|error| PreferencesError::Json(error.to_string()))
    }

    /// Decodes bounded version 1 JSON, rejecting unknown and duplicate fields.
    pub fn decode(bytes: &[u8]) -> Result<Self, PreferencesError> {
        if bytes.len() > MAX_PREFERENCES_BYTES {
            return Err(PreferencesError::InputTooLarge);
        }
        let file: PreferencesFile = serde_json::from_slice(bytes)
            .map_err(|error| PreferencesError::Json(error.to_string()))?;
        if file.format != "openunity.preferences" {
            return Err(PreferencesError::UnsupportedFormat);
        }
        if file.version != 1 {
            return Err(PreferencesError::UnsupportedVersion(file.version));
        }
        let preferences = Self {
            grid_visible: file.grid_visible,
            snap_step: file.snap_step,
            history_capacity: file.history_capacity,
        };
        preferences.validate()?;
        Ok(preferences)
    }

    /// Reads an optional regular file, returning `None` only for a missing file.
    ///
    /// Path validation rejects symlink components and missing parents. Read
    /// failures and invalid documents remain errors; they never become defaults.
    /// Bytes are bounded before parsing or allocating an unbounded buffer.
    pub fn load_optional(
        path: impl AsRef<Path>,
        policy: &WritePolicy,
    ) -> Result<Option<Self>, PreferencesError> {
        let path = path.as_ref();
        policy.check_read(path)?;
        let metadata = match fs::symlink_metadata(path) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(read_error(path, error)),
        };
        if !metadata.is_file() {
            return Err(read_error(path, "preferences must be a regular file"));
        }
        if metadata.len() > MAX_PREFERENCES_BYTES as u64 {
            return Err(PreferencesError::InputTooLarge);
        }
        let mut bytes = Vec::new();
        File::open(path)
            .and_then(|file| {
                file.take(MAX_PREFERENCES_BYTES as u64 + 1)
                    .read_to_end(&mut bytes)
            })
            .map_err(|error| read_error(path, error))?;
        Self::decode(&bytes).map(Some)
    }

    /// Atomically saves to an independent location permitted by `policy`.
    ///
    /// Existing files must decode as valid, supported preferences before they
    /// can be replaced. Rejection preserves the existing document.
    pub fn save(
        &self,
        path: impl AsRef<Path>,
        policy: &WritePolicy,
    ) -> Result<(), PreferencesError> {
        let bytes = self.encode()?;
        policy.atomic_write(path, &bytes, |existing| {
            Self::decode(existing)
                .map(|_| ())
                .map_err(|error| error.to_string())
        })?;
        Ok(())
    }
}

fn read_error(path: &Path, error: impl fmt::Display) -> PreferencesError {
    PreferencesError::Io {
        path: path.to_path_buf(),
        message: error.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT_DIRECTORY: AtomicU64 = AtomicU64::new(0);

    struct TestDirectory(PathBuf);

    impl TestDirectory {
        fn new() -> Self {
            let path = std::env::temp_dir().canonicalize().unwrap().join(format!(
                "openunity-preferences-{}-{}",
                std::process::id(),
                NEXT_DIRECTORY.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir(&path).unwrap();
            Self(path)
        }
    }

    impl Drop for TestDirectory {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn defaults_and_boundary_settings_round_trip() {
        assert_eq!(
            Preferences::default(),
            Preferences {
                grid_visible: true,
                snap_step: 0.5,
                history_capacity: 100,
            }
        );
        for preferences in [
            Preferences::default(),
            Preferences {
                grid_visible: false,
                snap_step: 1000.0,
                history_capacity: 1024,
            },
            Preferences {
                grid_visible: true,
                snap_step: f32::MIN_POSITIVE,
                history_capacity: 0,
            },
        ] {
            assert_eq!(
                Preferences::decode(&preferences.encode().unwrap()).unwrap(),
                preferences
            );
        }
    }

    #[test]
    fn invalid_settings_cannot_be_encoded() {
        for snap_step in [0.0, -0.1, 1000.1, f32::NAN, f32::INFINITY] {
            assert_eq!(
                Preferences {
                    snap_step,
                    ..Preferences::default()
                }
                .encode(),
                Err(PreferencesError::InvalidSnapStep)
            );
        }
        assert_eq!(
            Preferences {
                history_capacity: 1025,
                ..Preferences::default()
            }
            .encode(),
            Err(PreferencesError::InvalidHistoryCapacity)
        );
    }

    #[test]
    fn malformed_unsupported_and_out_of_bounds_documents_are_rejected() {
        let valid = r#"{"format":"openunity.preferences","version":1,"grid_visible":true,"snap_step":0.5,"history_capacity":100}"#;
        for malformed in [
            "{".to_string(),
            valid.replace("\"grid_visible\":true,", ""),
            valid.replace(
                "\"grid_visible\":true",
                "\"grid_visible\":true,\"grid_visible\":false",
            ),
            valid.replace("\"grid_visible\":true", "\"grid_visible\":true,\"extra\":0"),
            valid.replace("\"grid_visible\":true", "\"grid_visible\":1"),
            valid.replace("\"snap_step\":0.5", "\"snap_step\":1e100"),
            valid.replace("\"history_capacity\":100", "\"history_capacity\":-1"),
            format!("{valid} false"),
        ] {
            assert!(
                Preferences::decode(malformed.as_bytes()).is_err(),
                "{malformed}"
            );
        }
        for (document, expected) in [
            (
                valid.replace("openunity.preferences", "other"),
                PreferencesError::UnsupportedFormat,
            ),
            (
                valid.replace("\"version\":1", "\"version\":2"),
                PreferencesError::UnsupportedVersion(2),
            ),
            (
                valid.replace("\"snap_step\":0.5", "\"snap_step\":0"),
                PreferencesError::InvalidSnapStep,
            ),
            (
                valid.replace("\"history_capacity\":100", "\"history_capacity\":1025"),
                PreferencesError::InvalidHistoryCapacity,
            ),
        ] {
            assert_eq!(Preferences::decode(document.as_bytes()), Err(expected));
        }
        assert_eq!(
            Preferences::decode(&vec![b' '; MAX_PREFERENCES_BYTES + 1]),
            Err(PreferencesError::InputTooLarge)
        );
    }

    #[test]
    fn optional_load_distinguishes_missing_from_unreadable_or_invalid() {
        let directory = TestDirectory::new();
        let policy = WritePolicy::new([]).unwrap();
        let path = directory.0.join("settings.json");
        assert_eq!(Preferences::load_optional(&path, &policy).unwrap(), None);
        assert!(
            Preferences::load_optional(directory.0.join("missing/settings.json"), &policy).is_err()
        );
        assert!(Preferences::load_optional(&directory.0, &policy).is_err());
        fs::write(&path, b"not preferences").unwrap();
        assert!(Preferences::load_optional(&path, &policy).is_err());
        File::create(&path)
            .unwrap()
            .set_len(MAX_PREFERENCES_BYTES as u64 + 1)
            .unwrap();
        assert_eq!(
            Preferences::load_optional(&path, &policy),
            Err(PreferencesError::InputTooLarge)
        );
    }

    #[test]
    fn saves_new_and_owned_documents_but_preserves_unrelated_or_invalid_files() {
        let directory = TestDirectory::new();
        let policy = WritePolicy::new([]).unwrap();
        let path = directory.0.join("settings.json");
        Preferences::default().save(&path, &policy).unwrap();
        let changed = Preferences {
            grid_visible: false,
            ..Preferences::default()
        };
        changed.save(&path, &policy).unwrap();
        assert_eq!(
            Preferences::load_optional(&path, &policy).unwrap(),
            Some(changed)
        );
        for original in [
            b"unrelated settings".to_vec(),
            br#"{"format":"openunity.preferences","version":1}"#.to_vec(),
            Preferences::default().encode().unwrap(),
        ] {
            fs::write(&path, &original).unwrap();
            if Preferences::decode(&original).is_ok() {
                let invalid = Preferences {
                    snap_step: f32::NAN,
                    ..changed
                };
                assert!(invalid.save(&path, &policy).is_err());
            } else {
                assert!(changed.save(&path, &policy).is_err());
            }
            assert_eq!(fs::read(&path).unwrap(), original);
        }
    }

    #[cfg(unix)]
    #[test]
    fn optional_load_rejects_symlink_even_when_destination_is_missing() {
        let directory = TestDirectory::new();
        let link = directory.0.join("settings-link.json");
        std::os::unix::fs::symlink(directory.0.join("missing.json"), &link).unwrap();
        assert!(Preferences::load_optional(&link, &WritePolicy::new([]).unwrap()).is_err());
    }
}
