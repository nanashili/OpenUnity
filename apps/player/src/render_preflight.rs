//! Scene and manifest admission completed before host or device creation.

use crate::arguments::RenderArguments;
use crate::render_manifest::RenderManifest;
use std::fs::File;
use std::io::Read;
use std::path::Path;

const MAX_MANIFEST_BYTES: usize = 16 * 1024 * 1024;

pub(super) struct PreparedRender {
    pub(super) plan_payload: Vec<u8>,
    pub(super) manifest: RenderManifest,
    pub(super) snapshot_owners: Vec<i64>,
}

pub(super) fn load(arguments: &RenderArguments) -> Result<PreparedRender, String> {
    let plan =
        engine_scene::import_project(arguments.project(), arguments.scene(), arguments.bindings())
            .map_err(|error| format!("import scene: {error}"))?;

    let manifest_bytes = read_bounded_manifest(arguments.manifest())?;
    let manifest = RenderManifest::parse_json(&manifest_bytes)
        .map_err(|error| format!("parse render manifest: {error:?}"))?;
    manifest
        .validate_bindings(&plan)
        .map_err(|error| format!("validate scene bindings: {error}"))?;

    let snapshot_owners = manifest.snapshot_owner_order();
    let plan_payload = engine_serialization::serialize_plan(&plan)
        .map_err(|error| format!("serialize scene plan: {error}"))?;

    Ok(PreparedRender {
        plan_payload,
        manifest,
        snapshot_owners,
    })
}

fn read_bounded_manifest(path: &Path) -> Result<Vec<u8>, String> {
    let file = File::open(path).map_err(|error| format!("open render manifest: {error}"))?;
    let mut bytes = Vec::new();
    let mut bounded = file.take((MAX_MANIFEST_BYTES + 1) as u64);
    bounded
        .read_to_end(&mut bytes)
        .map_err(|error| format!("read render manifest: {error}"))?;
    if bytes.len() > MAX_MANIFEST_BYTES {
        return Err("render manifest exceeds 16 MiB".into());
    }
    std::str::from_utf8(&bytes)
        .map_err(|error| format!("render manifest is not UTF-8: {error}"))?;
    Ok(bytes)
}
