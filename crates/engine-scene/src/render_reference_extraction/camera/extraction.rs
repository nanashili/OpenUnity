use super::{
    CameraSourceProperties, CameraSourceValue, TransformSourcePose,
    fields::{capture_camera_properties, capture_transform_pose},
    identity::resolve_camera_identity,
};
use crate::{
    Result, SceneError,
    yaml::{self, Document},
};
use std::collections::BTreeMap;

const CAMERA_CLASS_ID: i64 = 20;

/// An uninterpreted Camera key and the parser-retained value shape.
#[derive(Clone, Debug, PartialEq)]
pub struct UninterpretedCameraField {
    /// The exact mapping key returned by the scene parser.
    pub key: String,
    /// The parsed scalar or collection shape. This is not a byte-exact YAML slice.
    pub source_value: CameraSourceValue,
}

/// Scene-local identifiers, source properties, and Transform pose for a Camera document.
#[derive(Clone, Debug, PartialEq)]
pub struct CameraSourceRecord {
    /// The serialized Camera component's local file identifier.
    pub component_file_id: i64,
    /// The serialized owning GameObject's local file identifier.
    pub game_object_file_id: i64,
    /// The mandatory serialized Transform's local file identifier.
    pub transform_file_id: i64,
    /// Captured Camera scalar fields and source representations.
    pub properties: CameraSourceProperties,
    /// Captured source pose from the owning Transform document.
    pub transform_pose: TransformSourcePose,
    /// Camera keys this extractor did not interpret, including their parsed source values.
    pub uninterpreted_fields: Vec<UninterpretedCameraField>,
}

/// Camera records in their scene-document order.
#[derive(Clone, Debug, PartialEq)]
pub struct SceneCameraSourceRecords {
    /// All Camera records found in the parsed scene, including disabled cameras.
    pub cameras: Vec<CameraSourceRecord>,
}

/// Extract bounded Camera source values while validating scene-local ownership links.
pub fn extract_scene_cameras(scene_bytes: &[u8]) -> Result<SceneCameraSourceRecords> {
    let documents = yaml::parse(scene_bytes)?;
    extract_scene_cameras_from_documents(&documents)
}

pub(crate) fn extract_scene_cameras_from_documents(
    documents: &[Document],
) -> Result<SceneCameraSourceRecords> {
    let owner_by_component = super::super::owners::owner_by_component(documents)?;
    let documents_by_id = index_documents(documents);
    let cameras = documents
        .iter()
        .filter(|document| document.class_id == CAMERA_CLASS_ID)
        .map(|camera| extract_camera(camera, &documents_by_id, &owner_by_component))
        .collect::<Result<Vec<_>>>()?;

    Ok(SceneCameraSourceRecords { cameras })
}

fn extract_camera(
    camera: &Document,
    documents_by_id: &BTreeMap<i64, &Document>,
    owner_by_component: &BTreeMap<i64, i64>,
) -> Result<CameraSourceRecord> {
    let identity = resolve_camera_identity(camera, documents_by_id, owner_by_component)?;
    let camera_fields = super::super::parsed_documents::component_fields(camera, "Camera")?;
    let transform = document_by_id(documents_by_id, identity.transform_file_id)?;
    let transform_fields =
        super::super::parsed_documents::component_fields(transform, "Transform")?;

    Ok(CameraSourceRecord {
        component_file_id: identity.camera_file_id,
        game_object_file_id: identity.game_object_file_id,
        transform_file_id: identity.transform_file_id,
        properties: capture_camera_properties(camera_fields),
        transform_pose: capture_transform_pose(transform_fields),
        uninterpreted_fields: uninterpreted_camera_fields(camera_fields),
    })
}

fn index_documents(documents: &[Document]) -> BTreeMap<i64, &Document> {
    documents
        .iter()
        .map(|document| (document.file_id, document))
        .collect()
}

fn document_by_id<'document>(
    documents_by_id: &BTreeMap<i64, &'document Document>,
    file_id: i64,
) -> Result<&'document Document> {
    documents_by_id.get(&file_id).copied().ok_or_else(|| {
        SceneError::new(format!(
            "camera extraction: Transform document {file_id} is missing"
        ))
    })
}

fn uninterpreted_camera_fields(
    fields: &BTreeMap<String, crate::yaml::Node>,
) -> Vec<UninterpretedCameraField> {
    fields
        .iter()
        .filter(|(key, _)| !is_interpreted_camera_key(key))
        .map(|(key, value)| UninterpretedCameraField {
            key: key.clone(),
            source_value: CameraSourceValue::from_node(value),
        })
        .collect()
}

fn is_interpreted_camera_key(key: &str) -> bool {
    matches!(
        key,
        "m_GameObject"
            | "m_Enabled"
            | "orthographic"
            | "orthographic size"
            | "field of view"
            | "near clip plane"
            | "far clip plane"
            | "m_Depth"
    )
}
