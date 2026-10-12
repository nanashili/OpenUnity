use super::{MetadataDiscoveryError, MetadataDiscoveryLimits};
use crate::{MetadataIndex, read};
use std::fs;
use std::io;
use std::path::{Component, Path, PathBuf};

/// Discovers GUID-bearing metadata below `root` into a complete index.
///
/// The walk preserves filesystem iteration order and performs no sorting or
/// canonicalization. Every yielded entry counts toward `max_entries` before
/// its type or contents are inspected. A failure discards the partial index.
///
/// # Errors
/// Returns a typed error for invalid paths, filesystem failures, symlinks,
/// unsupported entries, configured limits, malformed metadata, or duplicate
/// GUID ownership.
pub fn discover_metadata_index(
    root: impl AsRef<Path>,
    limits: MetadataDiscoveryLimits,
) -> Result<MetadataIndex, MetadataDiscoveryError> {
    let root = root.as_ref().to_owned();
    validate_root(&root)?;

    let mut index = MetadataIndex::new();
    let mut entry_count = 0;
    scan_directory(&root, &root, 0, &limits, &mut entry_count, &mut index)?;
    Ok(index)
}

fn validate_root(root: &Path) -> Result<(), MetadataDiscoveryError> {
    if !root.is_absolute()
        || root
            .components()
            .any(|component| matches!(component, Component::CurDir | Component::ParentDir))
        || !root.is_dir()
    {
        return Err(MetadataDiscoveryError::InvalidRoot(root.to_owned()));
    }

    validate_components(root, root)?;
    let metadata = fs::symlink_metadata(root).map_err(|source| io_error(root, source))?;
    if !metadata.is_dir() {
        return Err(MetadataDiscoveryError::InvalidRoot(root.to_owned()));
    }
    Ok(())
}

fn scan_directory(
    root: &Path,
    directory: &Path,
    depth: usize,
    limits: &MetadataDiscoveryLimits,
    entry_count: &mut usize,
    index: &mut MetadataIndex,
) -> Result<(), MetadataDiscoveryError> {
    if depth > limits.max_depth {
        return Err(MetadataDiscoveryError::DepthLimit(directory.to_owned()));
    }

    for entry in fs::read_dir(directory).map_err(|source| io_error(directory, source))? {
        let entry = entry.map_err(|source| io_error(directory, source))?;
        count_entry(directory, limits.max_entries, entry_count)?;

        let path = confined(root, &entry.path())?;
        let kind = entry
            .file_type()
            .map_err(|source| io_error(&path, source))?;
        if kind.is_dir() {
            scan_directory(root, &path, depth + 1, limits, entry_count, index)?;
        } else if !kind.is_file() {
            return Err(MetadataDiscoveryError::UnsupportedEntryType(path));
        } else if path
            .extension()
            .is_some_and(|extension| extension == "meta")
        {
            insert_metadata(root, &path, index)?;
        }
    }
    Ok(())
}

fn count_entry(
    directory: &Path,
    max_entries: usize,
    entry_count: &mut usize,
) -> Result<(), MetadataDiscoveryError> {
    *entry_count =
        entry_count
            .checked_add(1)
            .ok_or_else(|| MetadataDiscoveryError::EntryCountLimit {
                path: directory.to_owned(),
                max_entries,
            })?;
    if *entry_count > max_entries {
        return Err(MetadataDiscoveryError::EntryCountLimit {
            path: directory.to_owned(),
            max_entries,
        });
    }
    Ok(())
}

fn insert_metadata(
    root: &Path,
    metadata_path: &Path,
    index: &mut MetadataIndex,
) -> Result<(), MetadataDiscoveryError> {
    let metadata = read(metadata_path).map_err(|source| MetadataDiscoveryError::Metadata {
        path: metadata_path.to_owned(),
        source,
    })?;
    let asset_path = metadata_path.with_extension("");
    confined(root, &asset_path)?;
    index
        .insert(asset_path, metadata)
        .map_err(MetadataDiscoveryError::DuplicateGuid)
}

fn confined(root: &Path, path: &Path) -> Result<PathBuf, MetadataDiscoveryError> {
    if path.as_os_str().is_empty()
        || path
            .components()
            .any(|component| matches!(component, Component::ParentDir))
    {
        return Err(MetadataDiscoveryError::InvalidPath(path.to_owned()));
    }
    if !path.starts_with(root) {
        return Err(MetadataDiscoveryError::OutsideRoot(path.to_owned()));
    }

    validate_components(root, path)?;
    Ok(path.to_owned())
}

fn validate_components(root: &Path, path: &Path) -> Result<(), MetadataDiscoveryError> {
    let mut checked = PathBuf::new();
    for component in path.components() {
        checked.push(component.as_os_str());
        let metadata =
            fs::symlink_metadata(&checked).map_err(|source| io_error(&checked, source))?;
        if metadata.file_type().is_symlink() {
            return Err(MetadataDiscoveryError::Symlink(checked));
        }
    }
    if !path.starts_with(root) {
        return Err(MetadataDiscoveryError::OutsideRoot(path.to_owned()));
    }
    Ok(())
}

fn io_error(path: &Path, source: io::Error) -> MetadataDiscoveryError {
    MetadataDiscoveryError::Io {
        path: path.to_owned(),
        source,
    }
}
