use crate::{MAX_DEPTH, MAX_INPUT_BYTES, MAX_OBJECTS, Result, SceneError};
use engine_assets::MetadataIndex;
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
pub(super) fn index(
    root: &Path,
    dir: &Path,
    depth: usize,
    count: &mut usize,
    index: &mut MetadataIndex,
) -> Result<()> {
    if depth > MAX_DEPTH {
        return Err(SceneError::new("project directory depth limit"));
    }
    for entry in fs::read_dir(dir).map_err(SceneError::new)? {
        let entry = entry.map_err(SceneError::new)?;
        *count += 1;
        if *count > MAX_OBJECTS {
            return Err(SceneError::new("project entry count limit"));
        }
        let path = confined(root, &entry.path())?;
        let kind = entry.file_type().map_err(SceneError::new)?;
        if kind.is_dir() {
            index_recurse(root, &path, depth + 1, count, index)?;
        } else if !kind.is_file() {
            return Err(SceneError::new("unsupported project file type"));
        } else if path.extension().is_some_and(|e| e == "meta") {
            let meta = engine_assets::read(&path).map_err(SceneError::new)?;
            let asset = path.with_extension("");
            confined(root, &asset)?;
            index.insert(asset, meta).map_err(SceneError::new)?;
        }
    }
    Ok(())
}
// Separate name avoids shadowing the recursive function with the index value.
fn index_recurse(
    root: &Path,
    dir: &Path,
    depth: usize,
    count: &mut usize,
    entries: &mut MetadataIndex,
) -> Result<()> {
    index(root, dir, depth, count, entries)
}
