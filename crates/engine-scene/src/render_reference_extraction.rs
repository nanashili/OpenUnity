//! Extracts scene-owned serialized mesh and material references.

mod owners;
mod pointers;

use crate::{
    SceneError,
    yaml::{Document, Node},
};
use std::collections::BTreeMap;

/// A serialized asset pointer parsed without resolving it to an asset.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SerializedAssetPointer {
    /// The signed local file identifier from the serialized pointer.
    pub file_id: i64,
    /// The raw GUID value, if the serialized pointer contains one.
    pub guid: Option<String>,
    /// The numeric serialized type hint, if present; it does not prove asset kind.
    pub type_hint: Option<i64>,
}

/// The parsed state of one serialized pointer field.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PointerField {
    /// The field is absent from the component document.
    Missing,
    /// The field contains an unquoted `null` or `~` scalar.
    ExplicitNull,
    /// The field contains a parsed pointer, including one whose `file_id` is zero.
    Pointer(SerializedAssetPointer),
}

/// The parsed state of a MeshRenderer material-array field.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MaterialField {
    /// The field is absent from the component document.
    Missing,
    /// The field contains an unquoted `null` or `~` scalar.
    ExplicitNull,
    /// The field contains parser-supported ordered pointer maps; empty means a present empty sequence.
    Slots(Vec<SerializedAssetPointer>),
}

/// Serialized mesh reference and owner identity for one MeshFilter document.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MeshFilterReference {
    /// The MeshFilter document's local file identifier.
    pub component_file_id: i64,
    /// The owning GameObject document's local file identifier.
    pub game_object_file_id: i64,
    /// The parsed state of the MeshFilter's `m_Mesh` field.
    pub mesh: PointerField,
}

/// Serialized material references and owner identity for one MeshRenderer document.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MeshRendererReference {
    /// The MeshRenderer document's local file identifier.
    pub component_file_id: i64,
    /// The owning GameObject document's local file identifier.
    pub game_object_file_id: i64,
    /// The parsed state and ordered slots of the MeshRenderer's `m_Materials` field.
    pub materials: MaterialField,
}

/// MeshFilter and MeshRenderer records in their respective scene-document order.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SceneRenderReferences {
    /// Extracted MeshFilter records in scene-document order.
    pub mesh_filters: Vec<MeshFilterReference>,
    /// Extracted MeshRenderer records in scene-document order.
    pub mesh_renderers: Vec<MeshRendererReference>,
}

/// Extract MeshFilter and MeshRenderer serialized references from bounded scene text.
pub fn extract_scene_render_references(
    scene_bytes: &[u8],
) -> std::result::Result<SceneRenderReferences, SceneError> {
    let documents = crate::yaml::parse(scene_bytes)?;
    let owner_by_component = owners::owner_by_component(&documents)?;
    let mut mesh_filters = Vec::new();
    let mut mesh_renderers = Vec::new();

    for document in &documents {
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
    let mesh = pointers::parse_mesh_field(fields, &context)?;

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
    let materials = pointers::parse_materials_field(fields, &context)?;

    Ok(MeshRendererReference {
        component_file_id: document.file_id,
        game_object_file_id,
        materials,
    })
}

fn component_fields<'a>(
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

fn component_owner(
    document: &Document,
    fields: &BTreeMap<String, Node>,
    owner_by_component: &BTreeMap<i64, i64>,
    kind: &str,
) -> std::result::Result<i64, SceneError> {
    let context = format!("{kind} document {} m_GameObject", document.file_id);
    let node = fields
        .get("m_GameObject")
        .ok_or_else(|| SceneError::new(format!("{context} is missing")))?;
    let declared_owner = owners::parse_local_owner(node, &context)?;
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
