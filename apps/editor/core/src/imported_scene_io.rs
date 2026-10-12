//! Strict independent `.ouimport` persistence, never a Unity scene exporter.

use std::fs::File;
use std::io::Read;
use std::path::Path;

use serde::Deserialize;
use serde_json::value::RawValue;

use crate::imported_scene::{ImportedEditorId, ImportedError, ImportedSnapshot, ImportedSource};
use crate::write_guard::WritePolicy;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Archive {
    format: String,
    version: u32,
    source: ImportedSource,
    next_id: u64,
    editor_ids: Vec<ImportedEditorId>,
    // Keep original JSON bytes: the strict scene parser must see duplicate keys.
    plan: Box<RawValue>,
}

/// Encode the complete validated independent state without a lossy projection.
pub fn encode(snapshot: &ImportedSnapshot) -> Result<Vec<u8>, ImportedError> {
    snapshot.validate()?;
    let bytes =
        serde_json::to_vec(snapshot).map_err(|e| ImportedError::new("encode imported scene", e))?;
    check_bytes(&bytes)?;
    Ok(bytes)
}

/// Decode strict bounded JSON and retain the exact canonical editor/file-ID mapping.
pub fn decode(bytes: &[u8]) -> Result<ImportedSnapshot, ImportedError> {
    check_bytes(bytes)?;
    let archive: Archive = serde_json::from_slice(bytes)
        .map_err(|e| ImportedError::new("decode imported scene", e))?;
    if archive.format != "openunity.imported-scene" || archive.version != 1 {
        return Err(ImportedError::new(
            "decode imported scene",
            "unsupported format or version",
        ));
    }
    let plan = engine_scene::parse_plan(archive.plan.get().as_bytes())
        .map_err(|e| ImportedError::new("decode imported plan", e))?;
    ImportedSnapshot::new(archive.source, archive.next_id, archive.editor_ids, plan)
}

fn check_bytes(bytes: &[u8]) -> Result<(), ImportedError> {
    if bytes.len() > engine_scene::MAX_INPUT_BYTES {
        return Err(ImportedError::new(
            "decode imported scene",
            "archive exceeds 16 MiB",
        ));
    }
    let mut depth = 0_usize;
    let mut quoted = false;
    let mut escaped = false;
    for &byte in bytes {
        if quoted {
            if escaped {
                escaped = false;
            } else if byte == b'\\' {
                escaped = true;
            } else if byte == b'"' {
                quoted = false;
            }
        } else {
            match byte {
                b'"' => quoted = true,
                b'{' | b'[' => {
                    depth += 1;
                    if depth > 68 {
                        return Err(ImportedError::new(
                            "decode imported scene",
                            "JSON nesting exceeds 68 levels",
                        ));
                    }
                }
                b'}' | b']' => depth = depth.saturating_sub(1),
                _ => (),
            }
        }
    }
    Ok(())
}
fn check_extension(path: &Path) -> Result<(), ImportedError> {
    if path.extension().is_some_and(|e| e == "ouimport") {
        Ok(())
    } else {
        Err(ImportedError::new(
            "imported scene I/O",
            "independent imported scenes require .ouimport",
        ))
    }
}

/// Read an independent bounded archive. Source availability is not required.
pub fn load(path: &Path, policy: &WritePolicy) -> Result<ImportedSnapshot, ImportedError> {
    check_extension(path)?;
    policy
        .check_read(path)
        .map_err(|e| ImportedError::new("load imported scene", e))?;
    let prior = std::fs::symlink_metadata(path)
        .map_err(|e| ImportedError::new("load imported scene", e))?;
    if !prior.is_file() || prior.len() > engine_scene::MAX_INPUT_BYTES as u64 {
        return Err(ImportedError::new(
            "load imported scene",
            "nonregular or oversized archive",
        ));
    }
    let file = File::open(path).map_err(|e| ImportedError::new("load imported scene", e))?;
    let metadata = file
        .metadata()
        .map_err(|e| ImportedError::new("load imported scene", e))?;
    if !metadata.is_file() || metadata.len() > engine_scene::MAX_INPUT_BYTES as u64 {
        return Err(ImportedError::new(
            "load imported scene",
            "nonregular or oversized archive",
        ));
    }
    let mut bytes = Vec::new();
    file.take(engine_scene::MAX_INPUT_BYTES as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| ImportedError::new("load imported scene", e))?;
    decode(&bytes)
}

/// Atomically save outside both checked protected roots and retained source provenance.
/// Save As callers additionally require a new path; replacement here validates ownership.
pub fn save(
    path: &Path,
    snapshot: &ImportedSnapshot,
    policy: &WritePolicy,
) -> Result<(), ImportedError> {
    check_extension(path)?;
    if snapshot.source().forbids_write(path)? {
        return Err(ImportedError::new(
            "save imported scene",
            "writes within original source provenance are forbidden",
        ));
    }
    let bytes = encode(snapshot)?;
    policy
        .atomic_write(path, &bytes, |existing| {
            decode(existing).map(|_| ()).map_err(|e| e.to_string())
        })
        .map_err(|e| ImportedError::new("save imported scene", e))
}
