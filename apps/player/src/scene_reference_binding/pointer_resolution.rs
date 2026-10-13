use super::binding_rows::convert_rows;
use super::pairing::{PairedRenderOwner, pair_render_components};
use super::types::{AssetBindingRow, PointerResolutionError, ResolvedDraw, ResolvedSceneBindings};
use engine_assets::{AssetObjectCatalog, AssetObjectReference, Guid, MetadataIndex, RenderAssets};
use engine_scene::{MaterialField, PointerField, SceneRenderReferences};
use std::collections::BTreeSet;
use std::str::FromStr;

/// Builds one catalog and resolves every draw owner against it.
pub fn resolve_scene_bindings(
    metadata: MetadataIndex,
    render_assets: RenderAssets,
    references: &SceneRenderReferences,
    draw_owners: &[i64],
    rows: &[AssetBindingRow],
) -> Result<ResolvedSceneBindings, PointerResolutionError> {
    let pairs = pair_render_components(references, draw_owners)
        .map_err(|error| failure(format!("{error}")))?;
    let converted = convert_rows(rows)?;
    let catalog = AssetObjectCatalog::try_new(metadata, render_assets, converted.catalog_rows)
        .map_err(|error| {
            failure(format!(
                "render-asset-reference: catalog construction failed: {error}"
            ))
        })?;

    let mut unused = converted.unused;
    let draws = resolve_draws(&catalog, &pairs, &mut unused)?;
    if let Some(reference) = converted
        .authored_references
        .into_iter()
        .find(|reference| unused.contains(reference))
    {
        return Err(failure(format!(
            "render-asset-reference: unused binding {}:{}",
            reference.guid(),
            reference.local_file_id()
        )));
    }
    Ok(ResolvedSceneBindings { catalog, draws })
}

fn resolve_draws(
    catalog: &AssetObjectCatalog,
    pairs: &[PairedRenderOwner<'_>],
    unused: &mut BTreeSet<AssetObjectReference>,
) -> Result<Vec<ResolvedDraw>, PointerResolutionError> {
    pairs
        .iter()
        .map(|pair| {
            let mesh = pair.mesh_filter;
            let renderer = pair.mesh_renderer;
            let mesh_ref =
                pointer_reference(&mesh.mesh, pair.owner, mesh.component_file_id, "m_Mesh")?;
            let mesh_index = resource_mesh_index(
                catalog,
                &mesh_ref,
                pair.owner,
                mesh.component_file_id,
                "m_Mesh",
            )?;

            let material_ref =
                material_reference(&renderer.materials, pair.owner, renderer.component_file_id)?;
            let material_index = resource_material_index(
                catalog,
                &material_ref,
                pair.owner,
                renderer.component_file_id,
                "m_Materials",
            )?;

            unused.remove(&mesh_ref);
            unused.remove(&material_ref);
            Ok(ResolvedDraw {
                owner: pair.owner,
                mesh_index,
                material_index,
            })
        })
        .collect()
}

fn pointer_reference(
    field: &PointerField,
    owner: i64,
    component: i64,
    name: &'static str,
) -> Result<AssetObjectReference, PointerResolutionError> {
    let pointer = match field {
        PointerField::Pointer(pointer) => pointer,
        PointerField::Missing => return Err(pointer_failure(owner, component, name, "missing")),
        PointerField::ExplicitNull => {
            return Err(pointer_failure(owner, component, name, "explicit-null"));
        }
    };
    reference_from_parts(
        pointer.guid.as_deref(),
        pointer.file_id,
        owner,
        component,
        name,
    )
}

fn material_reference(
    field: &MaterialField,
    owner: i64,
    component: i64,
) -> Result<AssetObjectReference, PointerResolutionError> {
    let pointer = match field {
        MaterialField::Slots(slots) if slots.len() == 1 => &slots[0],
        MaterialField::Slots(_) => {
            return Err(pointer_failure(
                owner,
                component,
                "m_Materials",
                "requires exactly one slot",
            ));
        }
        MaterialField::Missing => {
            return Err(pointer_failure(owner, component, "m_Materials", "missing"));
        }
        MaterialField::ExplicitNull => {
            return Err(pointer_failure(
                owner,
                component,
                "m_Materials",
                "explicit-null",
            ));
        }
    };
    reference_from_parts(
        pointer.guid.as_deref(),
        pointer.file_id,
        owner,
        component,
        "m_Materials",
    )
}

fn reference_from_parts(
    guid: Option<&str>,
    local_file_id: i64,
    owner: i64,
    component: i64,
    field: &'static str,
) -> Result<AssetObjectReference, PointerResolutionError> {
    let guid = guid.ok_or_else(|| pointer_failure(owner, component, field, "missing GUID"))?;
    let guid = Guid::from_str(guid)
        .map_err(|error| pointer_failure(owner, component, field, error.to_string()))?;
    AssetObjectReference::new(guid, local_file_id)
        .map_err(|error| pointer_failure(owner, component, field, error.to_string()))
}

fn resource_mesh_index(
    catalog: &AssetObjectCatalog,
    reference: &AssetObjectReference,
    owner: i64,
    component: i64,
    field: &'static str,
) -> Result<u32, PointerResolutionError> {
    let mesh = catalog
        .resolve_mesh(reference)
        .map_err(|error| lookup_failure(owner, component, field, reference, error.to_string()))?;
    index_of(
        catalog
            .render_assets()
            .meshes()
            .iter()
            .map(|value| value.id()),
        mesh.id(),
    )
    .ok_or_else(|| {
        lookup_failure(
            owner,
            component,
            field,
            reference,
            "resolved mesh is absent from catalog snapshot",
        )
    })
}

fn resource_material_index(
    catalog: &AssetObjectCatalog,
    reference: &AssetObjectReference,
    owner: i64,
    component: i64,
    field: &'static str,
) -> Result<u32, PointerResolutionError> {
    let material = catalog
        .resolve_material(reference)
        .map_err(|error| lookup_failure(owner, component, field, reference, error.to_string()))?;
    index_of(
        catalog
            .render_assets()
            .materials()
            .iter()
            .map(|value| value.id()),
        material.id(),
    )
    .ok_or_else(|| {
        lookup_failure(
            owner,
            component,
            field,
            reference,
            "resolved material is absent from catalog snapshot",
        )
    })
}

fn index_of(ids: impl Iterator<Item = u32>, wanted: u32) -> Option<u32> {
    ids.enumerate()
        .find_map(|(index, id)| (id == wanted).then(|| u32::try_from(index).ok()).flatten())
}

fn pointer_failure(
    owner: i64,
    component: i64,
    field: &'static str,
    detail: impl Into<String>,
) -> PointerResolutionError {
    failure(format!(
        "scene-reference: owner {owner} component {component} field {field}: {}",
        detail.into()
    ))
}

fn lookup_failure(
    owner: i64,
    component: i64,
    field: &'static str,
    reference: &AssetObjectReference,
    detail: impl Into<String>,
) -> PointerResolutionError {
    failure(format!(
        "render-asset-reference: owner {owner} component {component} field {field} reference {}:{}: {}",
        reference.guid(),
        reference.local_file_id(),
        detail.into()
    ))
}

fn failure(detail: String) -> PointerResolutionError {
    PointerResolutionError::new(detail)
}
