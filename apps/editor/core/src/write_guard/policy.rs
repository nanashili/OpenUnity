//! Protect source roots and orchestrate validated independent-document writes.

use super::commit::atomic_commit;
use super::errors::{WriteError, error};
use super::ownership::validate_existing;
use super::paths::inspect_path;
use std::fs::{self, Metadata};
use std::path::{Path, PathBuf};

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
    /// The parent must exist. An existing target must be a validated owned regular
    /// document at most 16 MiB; a new target never clobbers a destination that appears
    /// before commit. Validation precedes temporary creation. Failures report retained
    /// temporary paths and explicitly distinguish a committed destination whose cleanup
    /// failed. Such an error does not mean that the destination was unchanged.
    pub fn atomic_write(
        &self,
        path: impl AsRef<Path>,
        bytes: &[u8],
        existing_validator: impl FnOnce(&[u8]) -> Result<(), String>,
    ) -> Result<(), WriteError> {
        let supplied = path.as_ref();
        let (destination, metadata) = self.destination(supplied)?;
        let replace = validate_existing(supplied, &destination, metadata, existing_validator)?;

        atomic_commit(&destination, bytes, replace).map_err(|cause| error(supplied, cause))
    }

    fn destination(&self, supplied: &Path) -> Result<(PathBuf, Option<Metadata>), WriteError> {
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
        Ok((destination, metadata))
    }
}
