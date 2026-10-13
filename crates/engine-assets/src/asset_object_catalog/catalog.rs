use super::catalog_errors::{AssetObjectCatalogBuildError, AssetObjectLookupError};
use super::reference::{
    AssetObjectBinding, AssetObjectClass, AssetObjectReference, AssetObjectTarget,
};
use crate::{MeshAsset, MetadataIndex, RenderAssets, UnlitMaterial};
use std::collections::BTreeMap;

const MAX_ASSET_OBJECT_BINDINGS: usize = 256;

/// Immutable GUID/local-file-ID bindings over one owned resource snapshot.
#[derive(Debug)]
pub struct AssetObjectCatalog {
    metadata: MetadataIndex,
    render_assets: RenderAssets,
    bindings: BTreeMap<AssetObjectReference, AssetObjectTarget>,
}

impl AssetObjectCatalog {
    /// Builds a complete catalog from explicit, validated resource bindings.
    ///
    /// The 257th yielded binding fails before its fields are inspected. Any
    /// failure drops the temporary map and returns no partial catalog.
    pub fn try_new(
        metadata: MetadataIndex,
        render_assets: RenderAssets,
        bindings: impl IntoIterator<Item = AssetObjectBinding>,
    ) -> Result<Self, AssetObjectCatalogBuildError> {
        let catalog_bindings = build_bindings(&metadata, &render_assets, bindings)?;

        Ok(Self {
            metadata,
            render_assets,
            bindings: catalog_bindings,
        })
    }

    /// Resolves a reference to the mesh owned by this catalog.
    pub fn resolve_mesh(
        &self,
        reference: &AssetObjectReference,
    ) -> Result<&MeshAsset, AssetObjectLookupError> {
        match self.resolve_target(reference, AssetObjectClass::Mesh)? {
            AssetObjectTarget::Mesh(id) => Ok(self
                .render_assets
                .meshes()
                .iter()
                .find(|mesh| mesh.id() == id)
                .expect("catalog construction validated every mesh target")),
            AssetObjectTarget::UnlitMaterial(_) => {
                unreachable!("resolve_target checked the requested mesh class")
            }
        }
    }

    /// Resolves a reference to the unlit material owned by this catalog.
    pub fn resolve_material(
        &self,
        reference: &AssetObjectReference,
    ) -> Result<&UnlitMaterial, AssetObjectLookupError> {
        match self.resolve_target(reference, AssetObjectClass::UnlitMaterial)? {
            AssetObjectTarget::UnlitMaterial(id) => Ok(self
                .render_assets
                .materials()
                .iter()
                .find(|material| material.id() == id)
                .expect("catalog construction validated every material target")),
            AssetObjectTarget::Mesh(_) => {
                unreachable!("resolve_target checked the requested material class")
            }
        }
    }

    fn resolve_target(
        &self,
        reference: &AssetObjectReference,
        expected: AssetObjectClass,
    ) -> Result<AssetObjectTarget, AssetObjectLookupError> {
        if self.metadata.asset_path(reference.guid()).is_none() {
            return Err(AssetObjectLookupError::MissingAssetGuid {
                reference: *reference,
            });
        }

        let target = self.bindings.get(reference).copied().ok_or(
            AssetObjectLookupError::MissingLocalFileId {
                reference: *reference,
            },
        )?;
        let actual = target.class();
        if actual != expected {
            return Err(AssetObjectLookupError::WrongAssetClass {
                reference: *reference,
                expected,
                actual,
            });
        }

        Ok(target)
    }
}

fn build_bindings(
    metadata: &MetadataIndex,
    render_assets: &RenderAssets,
    bindings: impl IntoIterator<Item = AssetObjectBinding>,
) -> Result<BTreeMap<AssetObjectReference, AssetObjectTarget>, AssetObjectCatalogBuildError> {
    let mut catalog_bindings = BTreeMap::new();

    for binding in bindings {
        if catalog_bindings.len() == MAX_ASSET_OBJECT_BINDINGS {
            return Err(AssetObjectCatalogBuildError::BindingLimitExceeded {
                max_bindings: MAX_ASSET_OBJECT_BINDINGS,
            });
        }

        insert_binding(metadata, render_assets, &mut catalog_bindings, binding)?;
    }

    Ok(catalog_bindings)
}

fn insert_binding(
    metadata: &MetadataIndex,
    render_assets: &RenderAssets,
    catalog_bindings: &mut BTreeMap<AssetObjectReference, AssetObjectTarget>,
    binding: AssetObjectBinding,
) -> Result<(), AssetObjectCatalogBuildError> {
    let reference = binding.reference();
    let target = binding.target();
    require_indexed_guid(metadata, reference)?;
    require_render_target(render_assets, reference, target)?;

    if catalog_bindings.contains_key(&reference) {
        return Err(AssetObjectCatalogBuildError::DuplicateReference { reference });
    }

    catalog_bindings.insert(reference, target);
    Ok(())
}

fn require_indexed_guid(
    metadata: &MetadataIndex,
    reference: AssetObjectReference,
) -> Result<(), AssetObjectCatalogBuildError> {
    if metadata.asset_path(reference.guid()).is_none() {
        return Err(AssetObjectCatalogBuildError::MissingAssetGuid { reference });
    }
    Ok(())
}

fn require_render_target(
    render_assets: &RenderAssets,
    reference: AssetObjectReference,
    target: AssetObjectTarget,
) -> Result<(), AssetObjectCatalogBuildError> {
    let exists = match target {
        AssetObjectTarget::Mesh(id) => render_assets.meshes().iter().any(|mesh| mesh.id() == id),
        AssetObjectTarget::UnlitMaterial(id) => render_assets
            .materials()
            .iter()
            .any(|material| material.id() == id),
    };
    if !exists {
        return Err(AssetObjectCatalogBuildError::MissingRenderTarget { reference, target });
    }
    Ok(())
}
