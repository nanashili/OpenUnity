use super::super::MAX_DRAWS;
use super::super::{CameraBinding, DrawBinding, RenderManifestError};
use super::json::{parse_unique_array, parse_unique_json};
use super::values::{object, parse_f32, parse_owner, parse_rgba, parse_u32, reject_unknown, take};
use engine_assets::RenderAssets;
use std::collections::BTreeSet;

pub(in crate::render_manifest) fn parse_camera(
    bytes: &[u8],
) -> Result<CameraBinding, RenderManifestError> {
    let value = parse_unique_json(bytes)?;
    let mut object = object(value, "camera")?;
    reject_unknown(
        &object,
        &["owner", "half_height", "near", "far", "clear_rgba8"],
        "camera",
    )?;
    let owner = parse_owner(take(&mut object, "owner")?, "camera.owner")?;
    let half_height = parse_f32(take(&mut object, "half_height")?, "camera.half_height")?;
    let near = parse_f32(take(&mut object, "near")?, "camera.near")?;
    let far = parse_f32(take(&mut object, "far")?, "camera.far")?;
    let clear_rgba8 = parse_rgba(take(&mut object, "clear_rgba8")?, "camera.clear_rgba8")?;
    if half_height <= 0.0 || near <= 0.0 || near >= far || clear_rgba8[3] != 255 {
        return Err(RenderManifestError::InvalidField("camera values"));
    }
    Ok(CameraBinding {
        owner,
        half_height,
        near,
        far,
        clear_rgba8,
    })
}

pub(in crate::render_manifest) fn parse_draws(
    bytes: &[u8],
) -> Result<Vec<RawDraw>, RenderManifestError> {
    let values = parse_unique_array(bytes, MAX_DRAWS, "draws")?;
    let mut owners = BTreeSet::new();
    values
        .into_iter()
        .map(|value| parse_draw(value, &mut owners))
        .collect()
}

fn parse_draw(
    value: serde_json::Value,
    owners: &mut BTreeSet<i64>,
) -> Result<RawDraw, RenderManifestError> {
    let mut object = object(value, "draw")?;
    reject_unknown(&object, &["owner", "mesh", "material", "enabled"], "draw")?;

    let owner = parse_owner(take(&mut object, "owner")?, "draw.owner")?;
    if !owners.insert(owner) {
        return Err(RenderManifestError::InvalidField("duplicate draw owner"));
    }

    let mesh_id = parse_u32(take(&mut object, "mesh")?, "draw.mesh")?;
    let material_id = parse_u32(take(&mut object, "material")?, "draw.material")?;
    let enabled = take(&mut object, "enabled")?
        .as_bool()
        .ok_or(RenderManifestError::InvalidField("draw.enabled"))?;

    Ok(RawDraw {
        owner,
        mesh_id,
        material_id,
        enabled,
    })
}

#[derive(Clone, Copy)]
pub(in crate::render_manifest) struct RawDraw {
    owner: i64,
    mesh_id: u32,
    material_id: u32,
    enabled: bool,
}

pub(in crate::render_manifest) fn resolve_draws(
    raw: Vec<RawDraw>,
    assets: &RenderAssets,
) -> Result<Vec<DrawBinding>, RenderManifestError> {
    raw.into_iter()
        .map(|draw| {
            let mesh_index = assets
                .meshes()
                .iter()
                .position(|mesh| mesh.id() == draw.mesh_id)
                .ok_or(RenderManifestError::InvalidField("draw.mesh reference"))?;
            let material_index = assets
                .materials()
                .iter()
                .position(|material| material.id() == draw.material_id)
                .ok_or(RenderManifestError::InvalidField("draw.material reference"))?;
            Ok(DrawBinding {
                owner: draw.owner,
                mesh_index: u32::try_from(mesh_index)
                    .map_err(|_| RenderManifestError::Limit("mesh index"))?,
                material_index: u32::try_from(material_index)
                    .map_err(|_| RenderManifestError::Limit("material index"))?,
                enabled: draw.enabled,
            })
        })
        .collect()
}
