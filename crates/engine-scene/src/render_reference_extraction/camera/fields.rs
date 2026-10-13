use super::source_value::{CameraField, CameraSourceValue};
use crate::{
    Result,
    importer::{flag, float, vector},
    yaml::Node,
};
use std::collections::BTreeMap;

/// Captured Camera source properties without defaults or runtime interpretation.
#[derive(Clone, Debug, PartialEq)]
pub struct CameraSourceProperties {
    /// The value serialized at `m_Enabled`, when present and parseable.
    pub enabled: CameraField<bool>,
    /// Whether the serialized `orthographic` value is true.
    pub orthographic: CameraField<bool>,
    /// No approved serialized aspect-key mapping exists. `None` does not claim source absence.
    pub aspect: Option<CameraField<f32>>,
    /// The value serialized at `orthographic size`.
    pub orthographic_size: CameraField<f32>,
    /// The value serialized at `field of view`.
    pub field_of_view: CameraField<f32>,
    /// The value serialized at `near clip plane`.
    pub near_clip_plane: CameraField<f32>,
    /// The value serialized at `far clip plane`.
    pub far_clip_plane: CameraField<f32>,
    /// The value serialized at `m_Depth`.
    pub depth: CameraField<f32>,
}

/// Serialized Transform pose values copied when extraction reads the source.
#[derive(Clone, Debug, PartialEq)]
pub struct TransformSourcePose {
    /// The Transform's local position, if represented as a supported vector.
    pub local_position: CameraField<[f32; 3]>,
    /// The Transform's local rotation, if represented as a supported quaternion.
    pub local_rotation: CameraField<[f32; 4]>,
    /// The Transform's local scale, if represented as a supported vector.
    pub local_scale: CameraField<[f32; 3]>,
}

pub(super) fn capture_camera_properties(fields: &BTreeMap<String, Node>) -> CameraSourceProperties {
    CameraSourceProperties {
        enabled: capture_field(fields, "m_Enabled", flag),
        orthographic: capture_field(fields, "orthographic", flag),
        aspect: None,
        orthographic_size: capture_field(fields, "orthographic size", float),
        field_of_view: capture_field(fields, "field of view", float),
        near_clip_plane: capture_field(fields, "near clip plane", float),
        far_clip_plane: capture_field(fields, "far clip plane", float),
        depth: capture_field(fields, "m_Depth", float),
    }
}

pub(super) fn capture_transform_pose(fields: &BTreeMap<String, Node>) -> TransformSourcePose {
    TransformSourcePose {
        local_position: capture_field(fields, "m_LocalPosition", vector::<3>),
        local_rotation: capture_field(fields, "m_LocalRotation", vector::<4>),
        local_scale: capture_field(fields, "m_LocalScale", vector::<3>),
    }
}

fn capture_field<T>(
    fields: &BTreeMap<String, Node>,
    key: &str,
    decode: impl FnOnce(&Node) -> Result<T>,
) -> CameraField<T> {
    let Some(node) = fields.get(key) else {
        return CameraField::Missing;
    };

    let source_value = CameraSourceValue::from_node(node);
    match decode(node) {
        Ok(value) => CameraField::Present {
            source_value,
            value,
        },
        Err(error) => CameraField::Invalid {
            source_value,
            diagnostic: format!("{key}: {error}"),
        },
    }
}
