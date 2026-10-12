//! Bounded metadata headers, metadata discovery, and render asset manifests.

mod guid;
mod json_parser;
mod manifest_wire;
mod mesh_asset;
mod metadata;
mod metadata_discovery;
mod metadata_index;
mod metadata_reader;
mod parts_wire;
mod render_assets;
mod unlit_material;

pub use guid::{Guid, InvalidGuid};
pub use mesh_asset::{AssetError, MeshAsset};
pub use metadata::{MAX_METADATA_BYTES, Metadata, MetadataError, parse};
pub use metadata_discovery::{
    MetadataDiscoveryError, MetadataDiscoveryLimits, discover_metadata_index,
};
pub use metadata_index::{DuplicateGuid, MetadataIndex};
pub use metadata_reader::read;
pub use render_assets::RenderAssets;
pub use unlit_material::UnlitMaterial;

#[cfg(test)]
mod render_asset_tests;

#[cfg(test)]
mod tests;
