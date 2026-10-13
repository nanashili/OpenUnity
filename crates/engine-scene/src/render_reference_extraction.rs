//! Extracts scene-owned serialized mesh and material references.

mod camera;
mod owners;
mod parsed_documents;
mod pointers;

use crate::SceneError;

pub use camera::{
    CameraField, CameraSourceProperties, CameraSourceRecord, CameraSourceValue,
    SceneCameraSourceRecords, TransformSourcePose, UninterpretedCameraField, extract_scene_cameras,
};

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
    parsed_documents::extract(&documents)
}

pub(crate) use parsed_documents::extract as extract_scene_render_references_from_documents;
