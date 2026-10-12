//! Filesystem guards for independently owned editor documents.
//!
//! Callers supply every source-project, pinned-installation and research root
//! that must be protected. Concurrent hostile filesystem mutation is outside
//! this policy's guarantee; it does not claim path-race hardening.

use std::fmt;
use std::fs::{self, File, Metadata, OpenOptions};
use std::io::{self, Read, Write};
use std::path::{Component, Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

/// Maximum existing document size accepted for ownership validation.
pub const MAX_EXISTING_BYTES: usize = 16 * 1024 * 1024;

static NEXT_TEMPORARY: AtomicU64 = AtomicU64::new(0);

/// A filesystem or ownership failure suitable for an editor diagnostic.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WriteError {
    /// The path whose operation was rejected or failed.
    pub path: PathBuf,
    /// Human-readable reason; existing documents are preserved on pre-commit failure.
    pub message: String,
}

impl fmt::Display for WriteError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}: {}", self.path.display(), self.message)
    }
}

impl std::error::Error for WriteError {}

/// Validated roots beneath which editor writes are forbidden.
#[derive(Clone, Debug)]
pub struct WritePolicy {
    protected_roots: Vec<PathBuf>,
}

impl WritePolicy {
    /// Protects existing, nonsymlink directories and all their descendants.
    ///
    /// Include source projects and the pinned reference/research locations.
    /// Invalid or missing roots fail construction rather than weakening protection.
    pub fn new(protected_roots: impl IntoIterator<Item = PathBuf>) -> Result<Self, WriteError> {
        let mut roots = Vec::new();
        for root in protected_roots {
            let (absolute, metadata) = inspect_path(&root)?;
            if !metadata.is_some_and(|metadata| metadata.is_dir()) {
                return Err(error(&root, "protected root must be an existing directory"));
            }
            let canonical = fs::canonicalize(&absolute).map_err(|cause| error(&root, cause))?;
            if !roots.contains(&canonical) {
                roots.push(canonical);
            }
        }
        Ok(Self {
            protected_roots: roots,
        })
    }

    /// Rejects parent traversal, symlink components, and missing parent directories.
    ///
    /// A missing final component is allowed for optional document reads. Read
    /// permission is independent of write protection; this does not open a file.
    pub fn check_read(&self, path: impl AsRef<Path>) -> Result<(), WriteError> {
        inspect_path(path.as_ref()).map(|_| ())
    }

    /// Atomically saves bytes outside all protected roots.
    ///
    /// The parent directory must already exist. A new target is committed
    /// without clobbering a destination that appeared in the meantime. An
    /// existing target must be a regular file at most 16 MiB, and the caller's
    /// validator must confirm its owned, supported document format before any
    /// temporary file is created. The validator is not called for a new target.
    pub fn atomic_write(
        &self,
        path: impl AsRef<Path>,
        bytes: &[u8],
        existing_validator: impl FnOnce(&[u8]) -> Result<(), String>,
    ) -> Result<(), WriteError> {
        let supplied = path.as_ref();
        let (absolute, metadata) = inspect_path(supplied)?;
        let parent = absolute.parent().ok_or_else(|| {
            error(
                supplied,
                "save target must have an existing parent directory",
            )
        })?;
        let canonical_parent = fs::canonicalize(parent).map_err(|cause| error(supplied, cause))?;
        let name = absolute
            .file_name()
            .ok_or_else(|| error(supplied, "save target must name a file"))?;
        let destination = canonical_parent.join(name);
        if self
            .protected_roots
            .iter()
            .any(|root| destination.starts_with(root))
        {
            return Err(error(
                supplied,
                "writes within a protected root are forbidden",
            ));
        }

        let replace = if let Some(metadata) = metadata {
            if !metadata.is_file() {
                return Err(error(
                    supplied,
                    "existing save target must be a regular file",
                ));
            }
            if metadata.len() > MAX_EXISTING_BYTES as u64 {
                return Err(error(
                    supplied,
                    "existing save target exceeds the 16 MiB limit",
                ));
            }
            let mut existing = Vec::new();
            File::open(&destination)
                .and_then(|file| {
                    file.take(MAX_EXISTING_BYTES as u64 + 1)
                        .read_to_end(&mut existing)
                })
                .map_err(|cause| error(supplied, cause))?;
            if existing.len() > MAX_EXISTING_BYTES {
                return Err(error(
                    supplied,
                    "existing save target exceeds the 16 MiB limit",
                ));
            }
            existing_validator(&existing).map_err(|reason| {
                error(
                    supplied,
                    format!("existing document is not owned: {reason}"),
                )
            })?;
            true
        } else {
            false
        };

        atomic_commit(&destination, bytes, replace).map_err(|cause| error(supplied, cause))
    }
}

fn error(path: &Path, message: impl fmt::Display) -> WriteError {
    WriteError {
        path: path.to_path_buf(),
        message: message.to_string(),
    }
}

fn inspect_path(path: &Path) -> Result<(PathBuf, Option<Metadata>), WriteError> {
    if path.as_os_str().is_empty() {
        return Err(error(path, "path must not be empty"));
    }
    if path.components().any(|part| part == Component::ParentDir) {
        return Err(error(path, "parent traversal is forbidden"));
    }
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .map_err(|cause| error(path, cause))?
            .join(path)
    };
    let mut partial = PathBuf::new();
    let components: Vec<_> = absolute.components().collect();
    let mut final_metadata = None;
    for (index, part) in components.iter().enumerate() {
        partial.push(part.as_os_str());
        let last = index + 1 == components.len();
        match fs::symlink_metadata(&partial) {
            Ok(metadata) => {
                if metadata.file_type().is_symlink() {
                    return Err(error(path, "symlink path components are forbidden"));
                }
                if !last && !metadata.is_dir() {
                    return Err(error(path, "parent path components must be directories"));
                }
                if last {
                    final_metadata = Some(metadata);
                }
            }
            Err(cause) if last && cause.kind() == io::ErrorKind::NotFound => {}
            Err(cause) => return Err(error(path, cause)),
        }
    }
    Ok((absolute, final_metadata))
}

struct TemporaryFile(PathBuf);

impl Drop for TemporaryFile {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.0);
    }
}

fn atomic_commit(destination: &Path, bytes: &[u8], replace: bool) -> io::Result<()> {
    let parent = destination
        .parent()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "save target has no parent"))?;
    let (temporary, mut file) = loop {
        let sequence = NEXT_TEMPORARY.fetch_add(1, Ordering::Relaxed);
        let path = parent.join(format!(".openunity-{}-{sequence}.tmp", std::process::id()));
        match OpenOptions::new().write(true).create_new(true).open(&path) {
            Ok(file) => break (TemporaryFile(path), file),
            Err(cause) if cause.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(cause) => return Err(cause),
        }
    };
    file.write_all(bytes)?;
    file.sync_all()?;
    drop(file);
    if replace {
        fs::rename(&temporary.0, destination)
    } else {
        fs::hard_link(&temporary.0, destination)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Directory(PathBuf);

    impl Directory {
        fn new() -> Self {
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
            let _ = fs::remove_dir_all(&self.0);
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
}
