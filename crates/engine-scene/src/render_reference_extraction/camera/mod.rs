//! Data-only extraction of serialized Camera fields and scene-local identity.

mod extraction;
mod fields;
mod identity;
mod source_value;

pub(crate) use extraction::extract_scene_cameras_from_documents;
pub use extraction::{
    CameraSourceRecord, SceneCameraSourceRecords, UninterpretedCameraField, extract_scene_cameras,
};
pub use fields::{CameraSourceProperties, TransformSourcePose};
pub use source_value::{CameraField, CameraSourceValue};
