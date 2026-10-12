//! Bounded versioned preference JSON.

use serde::{Deserialize, Serialize};

use super::{MAX_PREFERENCES_BYTES, Preferences, PreferencesError};

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
}
