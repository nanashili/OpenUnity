use crate::{MAX_METADATA_BYTES, Metadata, MetadataError, parse};
use std::fs::{self, File};
use std::io::{self, Read};
use std::path::{Component, Path, PathBuf};

/// Reads an explicit metadata file without importing or modifying assets.
///
/// All supplied path components, including ancestors, must be nonsymlinks. Empty
/// paths and parent traversal are rejected. At most [`MAX_METADATA_BYTES`] are
/// read. Concurrent hostile filesystem mutation is outside this reader's
/// guarantee; filesystem checks do not provide an atomic sandbox boundary.
///
/// # Errors
/// Reports missing metadata, symlinks, nonfiles, access failures, or any error
/// from [`parse`].
pub fn read(path: impl AsRef<Path>) -> Result<Metadata, MetadataError> {
    let path = path.as_ref();
    validate_metadata_path(path)?;
    let bytes = read_bounded_metadata(path)?;
    parse(&bytes)
}

fn validate_metadata_path(path: &Path) -> Result<(), MetadataError> {
    if path.as_os_str().is_empty()
        || path
            .components()
            .any(|component| matches!(component, Component::ParentDir))
    {
        return Err(MetadataError::InvalidPath(path.to_owned()));
    }
    let absolute = if path.is_absolute() {
        path.to_owned()
    } else {
        std::env::current_dir()
            .map_err(|source| io_error(path, source))?
            .join(path)
    };
    let mut checked = PathBuf::new();
    let mut components = absolute.components().peekable();
    while let Some(component) = components.next() {
        checked.push(component.as_os_str());
        let metadata =
            fs::symlink_metadata(&checked).map_err(|source| io_error(&checked, source))?;
        if metadata.file_type().is_symlink() {
            return Err(MetadataError::Symlink(checked));
        }
        // Reject special files before opening: opening a FIFO could block.
        if components.peek().is_none() && !metadata.is_file() {
            return Err(MetadataError::NotRegularFile(path.to_owned()));
        }
    }

    Ok(())
}

fn read_bounded_metadata(path: &Path) -> Result<Vec<u8>, MetadataError> {
    let file = File::open(path).map_err(|source| io_error(path, source))?;
    let metadata = file.metadata().map_err(|source| io_error(path, source))?;
    if !metadata.is_file() {
        return Err(MetadataError::NotRegularFile(path.to_owned()));
    }
    if metadata.len() > MAX_METADATA_BYTES as u64 {
        return Err(MetadataError::TooLarge);
    }
    let mut bytes = Vec::new();
    (&file)
        .take(MAX_METADATA_BYTES as u64)
        .read_to_end(&mut bytes)
        .map_err(|source| io_error(path, source))?;
    if file
        .metadata()
        .map_err(|source| io_error(path, source))?
        .len()
        > MAX_METADATA_BYTES as u64
    {
        return Err(MetadataError::TooLarge);
    }
    Ok(bytes)
}

fn io_error(path: &Path, source: io::Error) -> MetadataError {
    if source.kind() == io::ErrorKind::NotFound {
        MetadataError::MissingMetadata(path.to_owned())
    } else {
        MetadataError::Io {
            path: path.to_owned(),
            source,
        }
    }
}
