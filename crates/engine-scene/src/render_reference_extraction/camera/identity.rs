use super::super::{
    owners,
    parsed_documents::{component_fields, component_owner},
};
use crate::{Result, SceneError, yaml::Document, yaml::Node};
use std::collections::BTreeMap;

const CAMERA_CLASS_ID: i64 = 20;
const GAME_OBJECT_CLASS_ID: i64 = 1;
const TRANSFORM_CLASS_ID: i64 = 4;

/// Scene-local identities that connect one serialized Camera to its owner and Transform.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct CameraIdentity {
    pub(super) camera_file_id: i64,
    pub(super) game_object_file_id: i64,
    pub(super) transform_file_id: i64,
}

/// Resolve one Camera after document IDs and GameObject memberships were validated.
pub(super) fn resolve_camera_identity(
    camera: &Document,
    documents_by_id: &BTreeMap<i64, &Document>,
    owner_by_component: &BTreeMap<i64, i64>,
) -> Result<CameraIdentity> {
    require_class(camera, CAMERA_CLASS_ID, "Camera")?;

    let camera_fields = component_fields(camera, "Camera")?;
    let game_object_file_id = component_owner(camera, camera_fields, owner_by_component, "Camera")?;
    let game_object = document_by_id(documents_by_id, game_object_file_id, "GameObject")?;
    require_class(game_object, GAME_OBJECT_CLASS_ID, "GameObject")?;

    let game_object_fields = component_fields(game_object, "GameObject")?;
    let component_ids = game_object_component_ids(game_object_fields, game_object_file_id)?;
    require_camera_membership(&component_ids, camera.file_id, game_object_file_id)?;

    let transform_file_id =
        mandatory_transform_id(&component_ids, game_object_file_id, documents_by_id)?;
    validate_transform_owner(
        transform_file_id,
        game_object_file_id,
        documents_by_id,
        owner_by_component,
    )?;

    Ok(CameraIdentity {
        camera_file_id: camera.file_id,
        game_object_file_id,
        transform_file_id,
    })
}

fn game_object_component_ids(
    fields: &BTreeMap<String, Node>,
    game_object_file_id: i64,
) -> Result<Vec<i64>> {
    let entries = fields
        .get("m_Component")
        .ok_or_else(|| {
            SceneError::new(format!(
                "camera identity: GameObject document {game_object_file_id} is missing m_Component"
            ))
        })?
        .seq()
        .map_err(|error| {
            SceneError::new(format!(
                "camera identity: GameObject document {game_object_file_id} m_Component: {error}"
            ))
        })?;

    entries
        .iter()
        .enumerate()
        .map(|(index, entry)| component_entry_file_id(entry, game_object_file_id, index))
        .collect()
}

fn component_entry_file_id(entry: &Node, owner_id: i64, index: usize) -> Result<i64> {
    let entry_fields = entry.map().map_err(|error| {
        SceneError::new(format!(
            "camera identity: GameObject document {owner_id} m_Component entry {index}: {error}"
        ))
    })?;

    if entry_fields.len() != 1 || !entry_fields.contains_key("component") {
        return Err(SceneError::new(format!(
            "camera identity: GameObject document {owner_id} m_Component entry {index} must contain only component"
        )));
    }

    let pointer = entry_fields.get("component").ok_or_else(|| {
        SceneError::new(format!(
            "camera identity: GameObject document {owner_id} m_Component entry {index} is missing component"
        ))
    })?;
    let context = format!("GameObject document {owner_id} m_Component entry {index} component");

    owners::parse_local_owner(pointer, &context)
}

fn require_camera_membership(
    component_ids: &[i64],
    camera_file_id: i64,
    game_object_file_id: i64,
) -> Result<()> {
    if !component_ids.contains(&camera_file_id) {
        return Err(SceneError::new(format!(
            "camera identity: GameObject document {game_object_file_id} does not list Camera document {camera_file_id}"
        )));
    }

    Ok(())
}

fn mandatory_transform_id(
    component_ids: &[i64],
    game_object_file_id: i64,
    documents_by_id: &BTreeMap<i64, &Document>,
) -> Result<i64> {
    let mut transform_file_id = None;

    for component_file_id in component_ids {
        let component = document_by_id(documents_by_id, *component_file_id, "component")?;
        if component.class_id != TRANSFORM_CLASS_ID {
            continue;
        }

        if transform_file_id.replace(*component_file_id).is_some() {
            return Err(SceneError::new(format!(
                "camera identity: GameObject document {game_object_file_id} has multiple Transform components"
            )));
        }
    }

    let transform_file_id = transform_file_id.ok_or_else(|| {
        SceneError::new(format!(
            "camera identity: GameObject document {game_object_file_id} is missing its Transform"
        ))
    })?;

    Ok(transform_file_id)
}

fn validate_transform_owner(
    transform_file_id: i64,
    game_object_file_id: i64,
    documents_by_id: &BTreeMap<i64, &Document>,
    owner_by_component: &BTreeMap<i64, i64>,
) -> Result<()> {
    let transform = document_by_id(documents_by_id, transform_file_id, "Transform")?;
    require_class(transform, TRANSFORM_CLASS_ID, "Transform")?;

    let transform_fields = component_fields(transform, "Transform")?;
    let node = transform_fields.get("m_GameObject").ok_or_else(|| {
        SceneError::new(format!(
            "camera identity: Transform document {transform_file_id} is missing m_GameObject"
        ))
    })?;
    let context = format!("Transform document {transform_file_id} m_GameObject");
    let declared_owner = owners::parse_local_owner(node, &context)?;

    let listed_owner = owner_by_component.get(&transform_file_id).ok_or_else(|| {
        SceneError::new(format!(
            "camera identity: Transform document {transform_file_id} is not listed by a GameObject"
        ))
    })?;
    if declared_owner != game_object_file_id || *listed_owner != game_object_file_id {
        return Err(SceneError::new(format!(
            "camera identity: Transform document {transform_file_id} owner mismatch: m_GameObject={declared_owner}, list={listed_owner}, expected={game_object_file_id}"
        )));
    }

    Ok(())
}

fn document_by_id<'document>(
    documents_by_id: &BTreeMap<i64, &'document Document>,
    file_id: i64,
    kind: &str,
) -> Result<&'document Document> {
    documents_by_id.get(&file_id).copied().ok_or_else(|| {
        SceneError::new(format!(
            "camera identity: {kind} document {file_id} is missing"
        ))
    })
}

fn require_class(document: &Document, expected_class_id: i64, kind: &str) -> Result<()> {
    if document.class_id != expected_class_id {
        return Err(SceneError::new(format!(
            "camera identity: {kind} document {} has classID {}, expected {expected_class_id}",
            document.file_id, document.class_id
        )));
    }

    Ok(())
}
