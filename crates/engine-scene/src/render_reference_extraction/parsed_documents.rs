use super::{MeshFilterReference, MeshRendererReference, SceneRenderReferences};
use crate::{
    SceneError,
    yaml::{Document, Node},
};
use std::collections::BTreeMap;

pub(crate) fn extract(
    documents: &[Document],
) -> std::result::Result<SceneRenderReferences, SceneError> {
    let owner_by_component = super::owners::owner_by_component(documents)?;
    let mut mesh_filters = Vec::new();
    let mut mesh_renderers = Vec::new();

    for document in documents {
        match document.class_id {
            33 => mesh_filters.push(mesh_filter(document, &owner_by_component)?),
            23 => mesh_renderers.push(mesh_renderer(document, &owner_by_component)?),
            _ => {}
        }
    }

    Ok(SceneRenderReferences {
        mesh_filters,
        mesh_renderers,
    })
}

fn mesh_filter(
    document: &Document,
    owner_by_component: &BTreeMap<i64, i64>,
) -> std::result::Result<MeshFilterReference, SceneError> {
    let fields = component_fields(document, "MeshFilter")?;
    let game_object_file_id = component_owner(document, fields, owner_by_component, "MeshFilter")?;
    let context = format!("MeshFilter.m_Mesh (document {})", document.file_id);
    let mesh = super::pointers::parse_mesh_field(fields, &context)?;

    Ok(MeshFilterReference {
        component_file_id: document.file_id,
        game_object_file_id,
        mesh,
    })
}

fn mesh_renderer(
    document: &Document,
    owner_by_component: &BTreeMap<i64, i64>,
) -> std::result::Result<MeshRendererReference, SceneError> {
    let fields = component_fields(document, "MeshRenderer")?;
    let game_object_file_id =
        component_owner(document, fields, owner_by_component, "MeshRenderer")?;
    let context = format!("MeshRenderer.m_Materials (document {})", document.file_id);
    let materials = super::pointers::parse_materials_field(fields, &context)?;

    Ok(MeshRendererReference {
        component_file_id: document.file_id,
        game_object_file_id,
        materials,
    })
}

pub(super) fn component_fields<'a>(
    document: &'a Document,
    kind: &str,
) -> std::result::Result<&'a BTreeMap<String, Node>, SceneError> {
    let context = format!("{kind} document {}", document.file_id);
    let top = document
        .node
        .map()
        .map_err(|error| SceneError::new(format!("{context} top-level mapping: {error}")))?;
    if top.len() != 1 {
        return Err(SceneError::new(format!(
            "{context} requires one top-level mapping"
        )));
    }
    let value = top
        .get(kind)
        .ok_or_else(|| SceneError::new(format!("{context} has wrong top-level mapping name")))?;
    value
        .map()
        .map_err(|error| SceneError::new(format!("{context} fields: {error}")))
}

pub(super) fn component_owner(
    document: &Document,
    fields: &BTreeMap<String, Node>,
    owner_by_component: &BTreeMap<i64, i64>,
    kind: &str,
) -> std::result::Result<i64, SceneError> {
    let context = format!("{kind} document {} m_GameObject", document.file_id);
    let node = fields
        .get("m_GameObject")
        .ok_or_else(|| SceneError::new(format!("{context} is missing")))?;
    let declared_owner = super::owners::parse_local_owner(node, &context)?;
    let listed_owner = owner_by_component.get(&document.file_id).ok_or_else(|| {
        SceneError::new(format!(
            "{kind} document {} is not listed by a GameObject",
            document.file_id
        ))
    })?;
    if declared_owner != *listed_owner {
        return Err(SceneError::new(format!(
            "{kind} document {} owner mismatch: m_GameObject={declared_owner}, list={listed_owner}",
            document.file_id
        )));
    }
    Ok(declared_owner)
}
