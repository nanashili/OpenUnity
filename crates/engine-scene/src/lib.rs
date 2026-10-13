//! Independently designed, bounded authored-project scene import.
//! Only the reviewed authored text dialect is accepted; unsupported input fails.
#![forbid(unsafe_code)]

mod importer;
mod render_binding_owners;
mod render_reference_extraction;
mod yaml;

pub use engine_serialization::{
    FieldPlan, FieldValue, GameObjectPlan, MAX_DEPTH, MAX_FIELDS, MAX_INPUT_BYTES, MAX_OBJECTS,
    MAX_STRING_BYTES, SceneError, ScenePlan, ScriptPlan, TransformPlan, parse_plan,
};
pub use importer::{
    ProjectRenderImport, ProjectRenderSourceImport, import_project,
    import_project_with_render_references, import_project_with_render_sources,
};
pub use render_binding_owners::validate_render_binding_owners;
pub use render_reference_extraction::{
    CameraField, CameraSourceProperties, CameraSourceRecord, CameraSourceValue, MaterialField,
    MeshFilterReference, MeshRendererReference, PointerField, SceneCameraSourceRecords,
    SceneRenderReferences, SerializedAssetPointer, TransformSourcePose, UninterpretedCameraField,
    extract_scene_cameras, extract_scene_render_references,
};

use engine_serialization::{parse_json, validate_text as text};
type Result<T> = std::result::Result<T, SceneError>;
