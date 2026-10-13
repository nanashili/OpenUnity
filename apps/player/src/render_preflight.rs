//! Scene and manifest admission completed before host or device creation.

use crate::arguments::RenderArguments;
use crate::render_manifest::version_selection::ManifestRoute;
use crate::render_manifest::{self, RenderManifest};
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
    let manifest = read_deferred_manifest(arguments.manifest());
    let route = manifest
        .bytes()
        .map(render_manifest::version_selection::select)
        .unwrap_or(ManifestRoute::Legacy);

    let (plan, render_manifest) = match route {
        ManifestRoute::Legacy => load_legacy(arguments, manifest)?,
        ManifestRoute::Version2 => load_version2(arguments, manifest.expect_bytes()?)?,
    };

    let snapshot_owners = render_manifest.snapshot_owner_order();
    let plan_payload = engine_serialization::serialize_plan(&plan)
        .map_err(|error| format!("serialize scene plan: {error}"))?;

    Ok(PreparedRender {
        plan_payload,
        manifest: render_manifest,
        snapshot_owners,
    })
}

fn load_legacy(
    arguments: &RenderArguments,
    manifest: DeferredManifest,
) -> Result<(engine_scene::ScenePlan, RenderManifest), String> {
    let plan =
        engine_scene::import_project(arguments.project(), arguments.scene(), arguments.bindings())
            .map_err(|error| format!("import scene: {error}"))?;
    let bytes = manifest.expect_bytes()?;
    let render_manifest = RenderManifest::parse_json(&bytes)
        .map_err(|error| format!("parse render manifest: {error:?}"))?;
    render_manifest
        .validate_bindings(&plan)
        .map_err(|error| format!("validate scene bindings: {error}"))?;

    Ok((plan, render_manifest))
}

fn load_version2(
    arguments: &RenderArguments,
    manifest_bytes: Vec<u8>,
) -> Result<(engine_scene::ScenePlan, RenderManifest), String> {
    let imported = engine_scene::import_project_with_render_references(
        arguments.project(),
        arguments.scene(),
        arguments.bindings(),
    )
    .map_err(|error| format!("import scene: {error}"))?;
    let (plan, render_manifest) = RenderManifest::parse_version2(&manifest_bytes, imported)
        .map_err(render_manifest::version2_preflight_error)?;
    render_manifest
        .validate_bindings(&plan)
        .map_err(|error| format!("validate scene bindings: {error}"))?;

    Ok((plan, render_manifest))
}

enum DeferredManifest {
    Ready(Vec<u8>),
    Failed(String),
}

impl DeferredManifest {
    fn bytes(&self) -> Option<&[u8]> {
        match self {
            Self::Ready(bytes) => Some(bytes),
            Self::Failed(_) => None,
        }
    }

    fn expect_bytes(self) -> Result<Vec<u8>, String> {
        match self {
            Self::Ready(bytes) => Ok(bytes),
            Self::Failed(error) => Err(error),
        }
    }
}

fn read_deferred_manifest(path: &Path) -> DeferredManifest {
    let file = match File::open(path) {
        Ok(file) => file,
        Err(error) => return DeferredManifest::Failed(format!("open render manifest: {error}")),
    };
    let mut bytes = Vec::new();
    let mut bounded = file.take((MAX_MANIFEST_BYTES + 1) as u64);
    if let Err(error) = bounded.read_to_end(&mut bytes) {
        return DeferredManifest::Failed(format!("read render manifest: {error}"));
    }
    if bytes.len() > MAX_MANIFEST_BYTES {
        return DeferredManifest::Failed("render manifest exceeds 16 MiB".into());
    }
    if let Err(error) = std::str::from_utf8(&bytes) {
        return DeferredManifest::Failed(format!("render manifest is not UTF-8: {error}"));
    }
    DeferredManifest::Ready(bytes)
}
