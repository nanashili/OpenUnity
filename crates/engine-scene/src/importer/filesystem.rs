use crate::{MAX_INPUT_BYTES, Result, SceneError};
use std::{
    fs::{self, File},
    io::Read,
    path::{Component, Path, PathBuf},
};

pub(super) fn confined(root: &Path, path: &Path) -> Result<PathBuf> {
    if path.components().any(|c| matches!(c, Component::ParentDir)) || path.as_os_str().is_empty() {
        return Err(SceneError::new("invalid project path"));
    }
    let absolute = if path.is_absolute() {
        path.to_owned()
    } else {
        root.join(path)
    };
    if !absolute.starts_with(root) {
        return Err(SceneError::new("path outside project root"));
    }
    let mut checked = PathBuf::new();
    for component in absolute.components() {
        checked.push(component.as_os_str());
        if fs::symlink_metadata(&checked)
            .map_err(SceneError::new)?
            .file_type()
            .is_symlink()
        {
            return Err(SceneError::new("symbolic project path"));
        }
    }
    Ok(absolute)
}
pub(super) fn read(path: &Path) -> Result<Vec<u8>> {
    let prior = fs::symlink_metadata(path).map_err(SceneError::new)?;
    if !prior.is_file() || prior.len() > MAX_INPUT_BYTES as u64 {
        return Err(SceneError::new("nonregular or oversized input"));
    }
    let file = File::open(path).map_err(SceneError::new)?;
    let m = file.metadata().map_err(SceneError::new)?;
    if !m.is_file() || m.len() > MAX_INPUT_BYTES as u64 {
        return Err(SceneError::new("nonregular or oversized input"));
    }
    let mut bytes = Vec::new();
    file.take(MAX_INPUT_BYTES as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(SceneError::new)?;
    if bytes.len() > MAX_INPUT_BYTES {
        return Err(SceneError::new("input grew beyond byte limit"));
    }
    Ok(bytes)
}
