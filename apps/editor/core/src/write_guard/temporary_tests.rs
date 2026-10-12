//! Real filesystem failure coverage for staged-file finalization.
//!
//! Deliberate pathname obstruction exercises diagnostics and ownership only;
//! concurrent hostile filesystem mutation remains outside the policy guarantee.

use super::temporary::TemporaryFile;
use super::tests::Directory;
use std::fs;

#[test]
fn failed_publish_retains_original_error_and_cleanup_failure() {
    let directory = Directory::new();
    let destination = directory.0.join("document");
    fs::write(&destination, b"existing destination").unwrap();
    let (mut temporary, file) = TemporaryFile::create(&directory.0).unwrap();
    drop(file);
    let retained = temporary.path().to_path_buf();

    let publication = temporary.publish(&destination, false);
    let initiating_failure = publication.as_ref().unwrap_err().to_string();
    fs::remove_file(&retained).unwrap();
    fs::create_dir(&retained).unwrap();
    let error = temporary.finish(publication).unwrap_err().to_string();

    assert!(error.contains(&initiating_failure), "{error}");
    assert!(error.contains("temporary cleanup also failed"), "{error}");
    assert!(error.contains(&retained.display().to_string()), "{error}");
    assert_eq!(fs::read(&destination).unwrap(), b"existing destination");
    assert!(retained.is_dir());
}

#[test]
fn committed_destination_is_reported_when_temporary_cleanup_fails() {
    let directory = Directory::new();
    let destination = directory.0.join("document");
    let (mut temporary, file) = TemporaryFile::create(&directory.0).unwrap();
    drop(file);
    let retained = temporary.path().to_path_buf();
    fs::write(&retained, b"committed document").unwrap();

    let publication = temporary.publish(&destination, false);
    assert!(publication.is_ok());
    fs::remove_file(&retained).unwrap();
    fs::create_dir(&retained).unwrap();
    let error = temporary.finish(publication).unwrap_err().to_string();

    assert!(
        error.contains("destination committed; temporary cleanup failed"),
        "{error}"
    );
    assert!(error.contains(&retained.display().to_string()), "{error}");
    assert_eq!(fs::read(&destination).unwrap(), b"committed document");
    assert!(retained.is_dir());
}
