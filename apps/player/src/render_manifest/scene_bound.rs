//! Strict version-two render-manifest wire parsing.

use super::version_selection::{BoundedJsonError, validate_unique_fragment};
use engine_assets::{AssetError, RenderAssets};
use serde::Deserialize;
use serde_json::value::RawValue;
use std::collections::BTreeSet;

const MAX_DRAWS: usize = 4096;
const MAX_BINDINGS: usize = 256;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum V2WireError {
    InvalidJson,
    DuplicateKey,
    InvalidSchema,
    UnknownField,
    InvalidField(&'static str),
    Asset(AssetError),
    Limit(&'static str),
}

#[derive(Clone, Debug)]
pub(crate) struct V2Manifest<'a> {
    pub(crate) assets: RenderAssets,
    pub(crate) camera: V2Camera,
    pub(crate) draws: Vec<V2Draw>,
    pub(crate) asset_bindings: Vec<V2AssetBinding>,
    _source: std::marker::PhantomData<&'a RawValue>,
}

#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct V2Camera {
    pub(crate) owner: i64,
    pub(crate) half_height: f32,
    pub(crate) near: f32,
    pub(crate) far: f32,
    pub(crate) clear_rgba8: [u8; 4],
}

#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct V2Draw {
    pub(crate) owner: i64,
    pub(crate) enabled: bool,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct V2AssetBinding {
    pub(crate) guid: String,
    pub(crate) local_file_id: i64,
    pub(crate) kind: String,
    pub(crate) resource_id: u32,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Root<'a> {
    schema_version: u32,
    #[serde(borrow)]
    meshes: &'a RawValue,
    #[serde(borrow)]
    materials: &'a RawValue,
    camera: V2Camera,
    draws: Vec<V2Draw>,
    asset_bindings: Vec<V2AssetBinding>,
}

pub(crate) fn parse(bytes: &[u8]) -> Result<V2Manifest<'_>, V2WireError> {
    validate_unique_fragment(bytes).map_err(map_bounded_json_error)?;

    classify_unknown_fields(bytes)?;
    let root: Root<'_> = serde_json::from_slice(bytes).map_err(|_| V2WireError::InvalidSchema)?;
    if root.schema_version != 2 {
        return Err(V2WireError::InvalidField("schema_version"));
    }
    validate_camera(root.camera)?;
    validate_draws(&root.draws)?;
    validate_asset_bindings(&root.asset_bindings)?;

    let assets = RenderAssets::parse_parts(
        1,
        root.meshes.get().as_bytes(),
        root.materials.get().as_bytes(),
    )
    .map_err(V2WireError::Asset)?;

    Ok(V2Manifest {
        assets,
        camera: root.camera,
        draws: root.draws,
        asset_bindings: root.asset_bindings,
        _source: std::marker::PhantomData,
    })
}

fn classify_unknown_fields(bytes: &[u8]) -> Result<(), V2WireError> {
    let root = parse_object(bytes)?;
    ensure_fields(
        &root,
        &[
            "schema_version",
            "meshes",
            "materials",
            "camera",
            "draws",
            "asset_bindings",
        ],
    )?;
    classify_object_field(
        root.get("camera").copied(),
        &["owner", "half_height", "near", "far", "clear_rgba8"],
    )?;
    classify_array_field(root.get("draws").copied(), &["owner", "enabled"])?;
    classify_array_field(
        root.get("asset_bindings").copied(),
        &["guid", "local_file_id", "kind", "resource_id"],
    )?;
    Ok(())
}

type RawObject<'a> = std::collections::BTreeMap<String, &'a RawValue>;

fn parse_object<'a>(bytes: &'a [u8]) -> Result<RawObject<'a>, V2WireError> {
    serde_json::from_slice(bytes).map_err(|_| V2WireError::InvalidSchema)
}

fn classify_object_field(field: Option<&RawValue>, allowed: &[&str]) -> Result<(), V2WireError> {
    let Some(field) = field else {
        return Ok(());
    };
    let object: RawObject<'_> =
        serde_json::from_str(field.get()).map_err(|_| V2WireError::InvalidSchema)?;
    ensure_fields(&object, allowed)
}

fn classify_array_field(field: Option<&RawValue>, allowed: &[&str]) -> Result<(), V2WireError> {
    let Some(field) = field else {
        return Ok(());
    };
    let objects: Vec<&RawValue> =
        serde_json::from_str(field.get()).map_err(|_| V2WireError::InvalidSchema)?;
    for object in objects {
        classify_object_field(Some(object), allowed)?;
    }
    Ok(())
}

fn ensure_fields(object: &RawObject<'_>, allowed: &[&str]) -> Result<(), V2WireError> {
    if object
        .keys()
        .any(|field| !allowed.contains(&field.as_str()))
    {
        return Err(V2WireError::UnknownField);
    }
    Ok(())
}

fn map_bounded_json_error(error: BoundedJsonError) -> V2WireError {
    match error {
        BoundedJsonError::InvalidUtf8 | BoundedJsonError::InvalidJson => V2WireError::InvalidJson,
        BoundedJsonError::DuplicateKey => V2WireError::DuplicateKey,
        BoundedJsonError::Limit(limit) => V2WireError::Limit(limit),
    }
}

fn validate_camera(camera: V2Camera) -> Result<(), V2WireError> {
    if camera.owner <= 0
        || !camera.half_height.is_finite()
        || camera.half_height <= 0.0
        || !camera.near.is_finite()
        || !camera.far.is_finite()
        || camera.near <= 0.0
        || camera.near >= camera.far
        || camera.clear_rgba8[3] != 255
    {
        return Err(V2WireError::InvalidField("camera values"));
    }
    Ok(())
}

fn validate_draws(draws: &[V2Draw]) -> Result<(), V2WireError> {
    if draws.len() > MAX_DRAWS {
        return Err(V2WireError::Limit("draws"));
    }
    let mut owners = BTreeSet::new();
    for draw in draws {
        if draw.owner <= 0 {
            return Err(V2WireError::InvalidField("draw.owner"));
        }
        if !owners.insert(draw.owner) {
            return Err(V2WireError::InvalidField("duplicate draw owner"));
        }
    }
    Ok(())
}

fn validate_asset_bindings(bindings: &[V2AssetBinding]) -> Result<(), V2WireError> {
    if bindings.len() > MAX_BINDINGS {
        return Err(V2WireError::Limit("asset_bindings"));
    }
    let mut keys = BTreeSet::new();
    for binding in bindings {
        if binding.guid.len() != 32
            || !binding.guid.bytes().all(|byte| byte.is_ascii_hexdigit())
            || binding.local_file_id == 0
            || !matches!(binding.kind.as_str(), "mesh" | "unlit_material")
            || binding.resource_id == 0
        {
            return Err(V2WireError::InvalidField("asset_bindings"));
        }
        let key = (binding.guid.to_ascii_lowercase(), binding.local_file_id);
        if !keys.insert(key) {
            return Err(V2WireError::InvalidField("duplicate asset binding"));
        }
    }
    Ok(())
}
