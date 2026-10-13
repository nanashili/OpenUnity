//! Bounded metadata headers, metadata discovery, and render asset manifests.

mod asset_object_catalog;
mod guid;
mod json_parser;
mod manifest_wire;
mod mesh_asset;
mod metadata;
mod metadata_discovery;
mod metadata_index;
mod metadata_reader;
mod obj_import;
mod obj_import_error;
mod parts_wire;
mod render_assets;
mod unlit_material;

pub use asset_object_catalog::{
    AssetObjectBinding, AssetObjectCatalog, AssetObjectCatalogBuildError, AssetObjectClass,
    AssetObjectLookupError, AssetObjectReference, AssetObjectTarget, InvalidReference,
    InvalidReferenceComponent,
};
pub use guid::{Guid, InvalidGuid};
pub use mesh_asset::{AssetError, MeshAsset};
pub use metadata::{MAX_METADATA_BYTES, Metadata, MetadataError, parse};
pub use metadata_discovery::{
    MetadataDiscoveryError, MetadataDiscoveryLimits, discover_metadata_index,
};
pub use metadata_index::{DuplicateGuid, MetadataIndex};
pub use metadata_reader::read;
pub use obj_import::parse_obj_mesh;
pub use obj_import_error::ObjImportError;
pub use render_assets::RenderAssets;
pub use unlit_material::UnlitMaterial;

#[cfg(test)]
mod render_asset_tests;

#[cfg(test)]
mod tests;
