use std::fs::File;
use std::io::Read;
use std::path::Path;

use crate::scene::Scene;
use crate::write_guard::{MAX_EXISTING_BYTES, WritePolicy};

use super::{SceneIoError, decode, encode};

fn check_extension(path: &Path, operation: &'static str) -> Result<(), SceneIoError> {
    if path
        .extension()
        .is_some_and(|extension| extension == "ouscene")
    {
        Ok(())
    } else {
        Err(SceneIoError::new(
            operation,
            "independent scenes require the .ouscene extension",
        )
        .at(operation, path))
    }
}

/// Read a bounded `.ouscene` document through the shared path guard.
pub fn load(path: impl AsRef<Path>, policy: &WritePolicy) -> Result<Scene, SceneIoError> {
    let path = path.as_ref();
    let operation = "load scene";
    check_extension(path, operation)?;
    policy
        .check_read(path)
        .map_err(|cause| SceneIoError::new(operation, cause).at(operation, path))?;
    let metadata = std::fs::metadata(path)
        .map_err(|cause| SceneIoError::new(operation, cause).at(operation, path))?;
    if !metadata.is_file() {
        return Err(
            SceneIoError::new(operation, "scene input must be a regular file").at(operation, path),
        );
    }
    if metadata.len() > MAX_EXISTING_BYTES as u64 {
        return Err(
            SceneIoError::new(operation, "document exceeds the 16 MiB limit").at(operation, path),
        );
    }
    let file = File::open(path)
        .map_err(|cause| SceneIoError::new(operation, cause).at(operation, path))?;
    let mut bytes = Vec::new();
    file.take(MAX_EXISTING_BYTES as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|cause| SceneIoError::new(operation, cause).at(operation, path))?;
    decode(&bytes).map_err(|cause| cause.at(operation, path))
}

/// Atomically save outside protected roots, replacing only a validated owned scene.
pub fn save(
    path: impl AsRef<Path>,
    scene: &Scene,
    policy: &WritePolicy,
) -> Result<(), SceneIoError> {
    let path = path.as_ref();
    let operation = "save scene";
    check_extension(path, operation)?;
    let bytes = encode(scene).map_err(|cause| cause.at(operation, path))?;
    policy
        .atomic_write(path, &bytes, |existing| {
            decode(existing)
                .map(|_| ())
                .map_err(|cause| cause.to_string())
        })
        .map_err(|cause| SceneIoError::new(operation, cause).at(operation, path))
}
