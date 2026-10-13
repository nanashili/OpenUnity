use std::fs;
use std::path::{Component, Path, PathBuf};

use super::PublishError;
use super::receipt_files::io_error;

pub(super) fn create_output_directory(output: &Path) -> Result<PathBuf, PublishError> {
    validate_components(output)?;

    let absolute = if output.is_absolute() {
        output.to_path_buf()
    } else {
        std::env::current_dir()
            .map_err(|source| io_error("resolve current directory", source))?
            .join(output)
    };
    let name = absolute.file_name().ok_or_else(|| {
        PublishError::InvalidOutputPath("missing final directory component".into())
    })?;
    if !matches!(
        absolute.components().next_back(),
        Some(Component::Normal(_))
    ) {
        return Err(PublishError::InvalidOutputPath(
            "invalid final component".into(),
        ));
    }

    let parent = absolute
        .parent()
        .ok_or_else(|| PublishError::InvalidOutputPath("missing parent".into()))?;
    inspect_parent(parent)?;

    let canonical_parent = fs::canonicalize(parent)
        .map_err(|source| io_error("canonicalize output parent", source))?;
    let directory = canonical_parent.join(name);
    fs::create_dir(&directory)
        .map_err(|source| io_error("exclusively create output directory", source))?;
    Ok(directory)
}

fn validate_components(path: &Path) -> Result<(), PublishError> {
    if path.as_os_str().is_empty() {
        return Err(PublishError::InvalidOutputPath("empty path".into()));
    }

    if has_lexical_dot_component(path) {
        return Err(PublishError::InvalidOutputPath(
            "dot or traversal component".into(),
        ));
    }

    for component in path.components() {
        if matches!(component, Component::CurDir | Component::ParentDir) {
            return Err(PublishError::InvalidOutputPath(
                "dot or traversal component".into(),
            ));
        }
    }
    Ok(())
}

fn has_lexical_dot_component(path: &Path) -> bool {
    path.as_os_str()
        .as_encoded_bytes()
        .split(is_path_separator)
        .any(|component| matches!(component, b"." | b".."))
}

#[cfg(windows)]
fn is_path_separator(byte: &u8) -> bool {
    matches!(*byte, b'/' | b'\\')
}

#[cfg(not(windows))]
fn is_path_separator(byte: &u8) -> bool {
    *byte == b'/'
}

fn inspect_parent(parent: &Path) -> Result<(), PublishError> {
    let mut current = PathBuf::new();
    for component in parent.components() {
        current.push(component.as_os_str());
        if matches!(component, Component::RootDir | Component::Prefix(_)) {
            continue;
        }
        let metadata = fs::symlink_metadata(&current)
            .map_err(|source| io_error("inspect output parent", source))?;
        if metadata.file_type().is_symlink() || is_reparse_point(&metadata) {
            return Err(PublishError::InvalidOutputPath(
                "symlink or reparse component".into(),
            ));
        }
        if !metadata.is_dir() {
            return Err(PublishError::InvalidOutputPath(
                "parent is not a directory".into(),
            ));
        }
    }
    Ok(())
}

#[cfg(windows)]
fn is_reparse_point(metadata: &fs::Metadata) -> bool {
    use std::os::windows::fs::MetadataExt;
    metadata.file_attributes() & 0x400 != 0
}

#[cfg(not(windows))]
fn is_reparse_point(_: &fs::Metadata) -> bool {
    false
}
