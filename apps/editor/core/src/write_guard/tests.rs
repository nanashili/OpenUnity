use super::commit::atomic_commit;
use super::*;
use std::fs::{self, File};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_TEMPORARY: AtomicU64 = AtomicU64::new(0);

pub(super) struct Directory(pub(super) PathBuf);

impl Directory {
    pub(super) fn new() -> Self {
        let base = fs::canonicalize(std::env::temp_dir()).unwrap();
        let sequence = NEXT_TEMPORARY.fetch_add(1, Ordering::Relaxed);
        let path = base.join(format!(
            "openunity-write-test-{}-{sequence}",
            std::process::id()
        ));
        fs::create_dir(&path).unwrap();
        Self(path)
    }
}

impl Drop for Directory {
    fn drop(&mut self) {
        if let Err(cause) = fs::remove_dir_all(&self.0) {
            if std::thread::panicking() {
                use std::io::Write;
                let _ = writeln!(
                    std::io::stderr().lock(),
                    "Write fixture cleanup failed: {cause}"
                );
            } else {
                panic!(
                    "Write fixture cleanup failed for {}: {cause}",
                    self.0.display()
                );
            }
        }
    }
}

fn owned(bytes: &[u8]) -> Result<(), String> {
    if bytes.starts_with(b"independent-owned-v1\n") {
        Ok(())
    } else {
        Err("unsupported format".into())
    }
}

#[test]
fn creates_new_and_replaces_validated_owned_documents() {
    let directory = Directory::new();
    let policy = WritePolicy::new([]).unwrap();
    let path = directory.0.join("document");
    policy.check_read(&path).unwrap();
    policy
        .atomic_write(&path, b"independent-owned-v1\nold", |_| {
            panic!("new target")
        })
        .unwrap();
    policy
        .atomic_write(&path, b"independent-owned-v1\nnew", owned)
        .unwrap();
    assert_eq!(fs::read(&path).unwrap(), b"independent-owned-v1\nnew");
    assert_eq!(fs::read_dir(&directory.0).unwrap().count(), 1);
}

#[test]
fn protects_roots_without_blocking_sibling_names() {
    let directory = Directory::new();
    let protected = directory.0.join("source");
    fs::create_dir(&protected).unwrap();
    let policy = WritePolicy::new([protected.clone()]).unwrap();
    assert!(
        policy
            .atomic_write(protected.join("new"), b"no", owned)
            .is_err()
    );
    assert!(policy.atomic_write(&protected, b"no", owned).is_err());
    policy
        .atomic_write(directory.0.join("source-other"), b"yes", owned)
        .unwrap();
    assert_eq!(fs::read_dir(&protected).unwrap().count(), 0);
    assert!(WritePolicy::new([directory.0.join("missing")]).is_err());
    assert!(WritePolicy::new([directory.0.join("source-other")]).is_err());
}

#[test]
fn unrelated_and_oversized_existing_files_survive_rejection() {
    let directory = Directory::new();
    let policy = WritePolicy::new([]).unwrap();
    let unrelated = directory.0.join("unrelated.ouscene");
    fs::write(&unrelated, b"unrelated original").unwrap();
    assert!(
        policy
            .atomic_write(&unrelated, b"replacement", owned)
            .is_err()
    );
    assert_eq!(fs::read(&unrelated).unwrap(), b"unrelated original");
    let large = directory.0.join("large");
    File::create(&large)
        .unwrap()
        .set_len(MAX_EXISTING_BYTES as u64 + 1)
        .unwrap();
    assert!(
        policy
            .atomic_write(&large, b"replacement", |_| panic!("oversized"))
            .is_err()
    );
    assert_eq!(
        fs::metadata(&large).unwrap().len(),
        MAX_EXISTING_BYTES as u64 + 1
    );
    assert_eq!(fs::read_dir(&directory.0).unwrap().count(), 2);
}

#[test]
fn rejects_parent_traversal_and_missing_parents_before_writing() {
    let directory = Directory::new();
    let policy = WritePolicy::new([]).unwrap();
    for path in [
        directory.0.join("../escape"),
        directory.0.join("missing/new"),
    ] {
        assert!(policy.check_read(&path).is_err());
        assert!(policy.atomic_write(path, b"no", owned).is_err());
    }
    assert_eq!(fs::read_dir(&directory.0).unwrap().count(), 0);
}

#[cfg(unix)]
#[test]
fn rejects_symlinks_in_roots_parents_and_final_components() {
    use std::os::unix::fs::symlink;
    let directory = Directory::new();
    let outside = Directory::new();
    let policy = WritePolicy::new([]).unwrap();
    symlink(&outside.0, directory.0.join("parent-link")).unwrap();
    symlink(outside.0.join("missing"), directory.0.join("final-link")).unwrap();
    assert!(WritePolicy::new([directory.0.join("parent-link")]).is_err());
    for path in [
        directory.0.join("parent-link/new"),
        directory.0.join("final-link"),
    ] {
        assert!(policy.check_read(&path).is_err());
        assert!(policy.atomic_write(path, b"no", owned).is_err());
    }
    assert_eq!(fs::read_dir(&outside.0).unwrap().count(), 0);
}

#[test]
fn failed_commit_cleans_temporary_file() {
    let directory = Directory::new();
    let policy = WritePolicy::new([]).unwrap();
    let destination = directory.0.join("document");
    fs::write(&destination, b"independent-owned-v1\nold").unwrap();
    // Force a real rename failure after ownership validation, without a
    // production-only fault injection API. Hostile mutation is not guaranteed.
    let result = policy.atomic_write(&destination, b"new", |existing| {
        owned(existing)?;
        fs::remove_file(&destination).unwrap();
        fs::create_dir(&destination).unwrap();
        Ok(())
    });
    assert!(result.is_err());
    assert!(destination.is_dir());
    assert_eq!(fs::read_dir(&directory.0).unwrap().count(), 1);
}

#[test]
fn new_commit_never_clobbers_an_existing_destination() {
    let directory = Directory::new();
    let destination = directory.0.join("document");
    fs::write(&destination, b"appeared before commit").unwrap();
    assert!(atomic_commit(&destination, b"new", false).is_err());
    assert_eq!(fs::read(&destination).unwrap(), b"appeared before commit");
    assert_eq!(fs::read_dir(&directory.0).unwrap().count(), 1);
}
