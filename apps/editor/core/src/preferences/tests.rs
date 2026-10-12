use std::fs::{self, File};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use crate::write_guard::WritePolicy;

use super::*;

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
        if let Err(cause) = fs::remove_dir_all(&self.0) {
            let message = format!("failed to remove fixture {}: {cause}", self.0.display());
            if std::thread::panicking() {
                use std::io::Write;
                // Reporting must not replace the initiating assertion panic,
                // including when the diagnostic stream itself is unavailable.
                let _ = writeln!(std::io::stderr().lock(), "{message}");
            } else {
                panic!("{message}");
            }
        }
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
