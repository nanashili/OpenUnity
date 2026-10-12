use super::json_parser::{MAX_JSON_DEPTH, parse_asset_json_with_depth_limit};
use super::manifest_wire::{BoundedVec, RawMaterial, RawMesh, RawMeshes};
use super::mesh_asset::AssetError;
use super::render_assets::MAX_MESHES;

pub(super) fn parse_meshes(bytes: &[u8]) -> Result<Vec<RawMesh>, AssetError> {
    let raw: RawMeshes = parse_asset_json_with_depth_limit(bytes, MAX_JSON_DEPTH - 1)?;
    Ok(raw.into_vec())
}

pub(super) fn parse_materials(bytes: &[u8]) -> Result<Vec<RawMaterial>, AssetError> {
    let raw: BoundedVec<RawMaterial, MAX_MESHES> =
        parse_asset_json_with_depth_limit(bytes, MAX_JSON_DEPTH - 1)?;
    Ok(raw.into_vec())
}
