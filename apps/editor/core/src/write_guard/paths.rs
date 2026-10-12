//! Inspect path components before any document read or write.

use super::errors::{WriteError, error};
use std::fs::{self, Metadata};
use std::io;
use std::path::{Component, Path, PathBuf};

pub(super) fn inspect_path(path: &Path) -> Result<(PathBuf, Option<Metadata>), WriteError> {
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
