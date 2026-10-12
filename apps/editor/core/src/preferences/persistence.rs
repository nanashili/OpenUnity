//! Guarded optional reads and atomic writes for preferences.

use std::fmt;
use std::fs::{self, File};
use std::io::{self, Read};
use std::path::Path;

use crate::write_guard::WritePolicy;

use super::{MAX_PREFERENCES_BYTES, Preferences, PreferencesError};

impl Preferences {
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
