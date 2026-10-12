//! Validated, independently versioned editor preferences and guarded persistence.

mod codec;
mod model;
mod persistence;

pub use model::{
    MAX_HISTORY_CAPACITY, MAX_PREFERENCES_BYTES, MAX_SNAP_STEP, Preferences, PreferencesError,
};

#[cfg(test)]
mod tests;
