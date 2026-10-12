use std::collections::BTreeSet;

use super::json_parser::{parse_asset_json, validate_asset_parts_bounds};
use super::manifest_wire::{RawManifest, RawMaterial};
pub use super::mesh_asset::AssetError;
use super::mesh_asset::MeshAsset;
use super::parts_wire::{parse_materials, parse_meshes};
use super::unlit_material::UnlitMaterial;

pub(crate) const MAX_MESHES: usize = 256;
pub(crate) const MAX_POSITIONS: usize = 262_144;
pub(crate) const MAX_INDICES: usize = 786_432;

#[derive(Clone, Debug)]
/// Validated meshes and materials from one render asset manifest.
pub struct RenderAssets {
    schema_version: u32,
    meshes: Vec<MeshAsset>,
    materials: Vec<UnlitMaterial>,
}

impl RenderAssets {
    /// Creates schema-version-one assets from already validated typed values.
    ///
    /// Values retain their supplied order. Mesh and material IDs must be
    /// unique within their own kind, while IDs may be reused across kinds.
    pub fn try_new(
        meshes: Vec<MeshAsset>,
        materials: Vec<UnlitMaterial>,
    ) -> Result<Self, AssetError> {
        validate_mesh_collection(&meshes)?;
        validate_material_collection(&materials)?;

        Ok(Self {
            schema_version: 1,
            meshes,
            materials,
        })
    }

    /// Parses only the asset portion of the bounded JSON manifest.
    pub fn parse_json(bytes: &[u8]) -> Result<Self, AssetError> {
        let raw: RawManifest = parse_asset_json(bytes)?;
        validate_schema_version(raw.schema_version)?;
        let meshes = build_meshes(raw.meshes.into_vec())?;
        let materials = build_materials(raw.materials.into_vec())?;

        Ok(Self {
            schema_version: raw.schema_version,
            meshes,
            materials,
        })
    }

    /// Parses borrowed mesh and material array fragments from one bounded manifest.
    ///
    /// The enclosing player parser must bound the original manifest to 16 MiB
    /// and depth 64. This method also caps combined fragment bytes at 16 MiB and
    /// limits each fragment to depth 63 beneath the manifest object.
    pub fn parse_parts(
        schema_version: u32,
        meshes_json: &[u8],
        materials_json: &[u8],
    ) -> Result<Self, AssetError> {
        validate_asset_parts_bounds(meshes_json, materials_json)?;
        validate_schema_version(schema_version)?;

        let meshes = build_meshes(parse_meshes(meshes_json)?)?;
        let materials = build_materials(parse_materials(materials_json)?)?;

        Ok(Self {
            schema_version,
            meshes,
            materials,
        })
    }

    /// Returns all validated meshes in manifest order.
    pub fn meshes(&self) -> &[MeshAsset] {
        &self.meshes
    }

    /// Returns all validated materials in manifest order.
    pub fn materials(&self) -> &[UnlitMaterial] {
        &self.materials
    }

    /// Returns the manifest schema version.
    pub fn schema_version(&self) -> u32 {
        self.schema_version
    }
}

fn validate_schema_version(schema_version: u32) -> Result<(), AssetError> {
    if schema_version == 1 {
        return Ok(());
    }

    Err(AssetError::Parse(
        "unsupported render asset manifest".into(),
    ))
}

fn build_meshes(
    raw_meshes: Vec<super::manifest_wire::RawMesh>,
) -> Result<Vec<MeshAsset>, AssetError> {
    let mut meshes = Vec::with_capacity(raw_meshes.len());
    let mut validator = MeshCollectionValidator::new(raw_meshes.len())?;

    for raw_mesh in raw_meshes {
        let mesh = MeshAsset::try_new(raw_mesh.id, raw_mesh.positions, raw_mesh.indices)?;
        validator.validate(&mesh)?;
        meshes.push(mesh);
    }

    Ok(meshes)
}

fn build_materials(raw_materials: Vec<RawMaterial>) -> Result<Vec<UnlitMaterial>, AssetError> {
    let mut materials = Vec::with_capacity(raw_materials.len());
    let mut validator = MaterialCollectionValidator::new(raw_materials.len())?;

    for raw_material in raw_materials {
        let material = UnlitMaterial::try_new(raw_material.id, raw_material.rgba8)?;
        validator.validate(&material)?;
        materials.push(material);
    }

    Ok(materials)
}

fn validate_mesh_collection(meshes: &[MeshAsset]) -> Result<(), AssetError> {
    let mut validator = MeshCollectionValidator::new(meshes.len())?;

    for mesh in meshes {
        validator.validate(mesh)?;
    }

    Ok(())
}

fn validate_material_collection(materials: &[UnlitMaterial]) -> Result<(), AssetError> {
    let mut validator = MaterialCollectionValidator::new(materials.len())?;

    for material in materials {
        validator.validate(material)?;
    }

    Ok(())
}

struct MeshCollectionValidator {
    ids: BTreeSet<u32>,
    positions: usize,
    indices: usize,
}

impl MeshCollectionValidator {
    fn new(count: usize) -> Result<Self, AssetError> {
        if count > MAX_MESHES {
            return Err(AssetError::Limit("render asset count"));
        }

        Ok(Self {
            ids: BTreeSet::new(),
            positions: 0,
            indices: 0,
        })
    }

    fn validate(&mut self, mesh: &MeshAsset) -> Result<(), AssetError> {
        if !self.ids.insert(mesh.id()) {
            return Err(AssetError::DuplicateId("mesh"));
        }

        self.positions = self
            .positions
            .checked_add(mesh.positions().len())
            .ok_or(AssetError::Limit("mesh positions"))?;
        if self.positions > MAX_POSITIONS {
            return Err(AssetError::Limit("mesh positions"));
        }

        self.indices = self
            .indices
            .checked_add(mesh.indices().len())
            .ok_or(AssetError::Limit("mesh indices"))?;
        if self.indices > MAX_INDICES {
            return Err(AssetError::Limit("mesh indices"));
        }

        Ok(())
    }
}

struct MaterialCollectionValidator {
    ids: BTreeSet<u32>,
}

impl MaterialCollectionValidator {
    fn new(count: usize) -> Result<Self, AssetError> {
        if count > MAX_MESHES {
            return Err(AssetError::Limit("render asset count"));
        }

        Ok(Self {
            ids: BTreeSet::new(),
        })
    }

    fn validate(&mut self, material: &UnlitMaterial) -> Result<(), AssetError> {
        if !self.ids.insert(material.id()) {
            return Err(AssetError::DuplicateId("material"));
        }

        Ok(())
    }
}
