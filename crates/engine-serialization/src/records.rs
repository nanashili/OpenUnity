use serde::{Deserialize, Serialize};

/// Owned scene graph, with local file identities distinct from native identities.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ScenePlan {
    /// Transfer format version, currently one.
    pub schema_version: u32,
    /// Diagnostic name, not a managed type or native identity.
    pub scene_name: String,
    /// Ordered root Transform file identities.
    pub roots: Vec<i64>,
    /// GameObject records.
    pub game_objects: Vec<GameObjectPlan>,
    /// Mandatory Transform records.
    pub transforms: Vec<TransformPlan>,
    /// Authored managed component records.
    pub scripts: Vec<ScriptPlan>,
}

/// Serialized GameObject state.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct GameObjectPlan {
    /// Local file identity.
    pub file_id: i64,
    /// Object name.
    pub name: String,
    /// Local activity flag.
    pub active: bool,
    /// Ordered component file identities, including the mandatory Transform.
    pub components: Vec<i64>,
}

/// Serialized local Transform state.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TransformPlan {
    /// Local file identity.
    pub file_id: i64,
    /// Owning GameObject file identity.
    pub owner: i64,
    /// Parent Transform file identity, or zero for a root.
    pub parent: i64,
    /// Ordered child Transform file identities.
    pub children: Vec<i64>,
    /// Local position, requiring finite float32 components.
    pub local_position: [f32; 3],
    /// Local rotation; native numerical constraints additionally apply.
    pub local_rotation: [f32; 4],
    /// Local scale; native numerical constraints additionally apply.
    pub local_scale: [f32; 3],
}

/// An explicitly GUID-bound managed component.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ScriptPlan {
    /// Local file identity.
    pub file_id: i64,
    /// Owning GameObject file identity.
    pub owner: i64,
    /// Validated metadata GUID.
    pub guid: String,
    /// Independently compiled managed assembly identity.
    pub assembly: String,
    /// Fully qualified managed type identity.
    pub type_name: String,
    /// Serialized Behaviour.enabled flag.
    pub enabled: bool,
    /// Explicit script execution order, checked against reflected metadata.
    pub execution_order: i32,
    /// Complete supported serialized instance fields.
    pub fields: Vec<FieldPlan>,
}

/// One strictly typed serialized field.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FieldPlan {
    /// Exact reflected field name.
    pub name: String,
    /// Typed value, without property setters or numeric coercion.
    pub value: FieldValue,
}

/// Supported independent transfer kinds; quaternion fields are not normalized.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "kind", content = "value", deny_unknown_fields)]
pub enum FieldValue {
    /// Signed 32-bit integer.
    #[serde(rename = "i32")]
    I32(i32),
    /// Finite 32-bit float.
    #[serde(rename = "f32")]
    F32(f32),
    /// Boolean.
    #[serde(rename = "bool")]
    Bool(bool),
    /// UTF-8 string.
    #[serde(rename = "string")]
    String(String),
    /// Three finite float components.
    #[serde(rename = "vec3")]
    Vec3([f32; 3]),
    /// Four finite raw float components.
    #[serde(rename = "quat")]
    Quat([f32; 4]),
    /// Local file identity, with zero explicitly denoting null.
    #[serde(rename = "reference")]
    Reference(i64),
}
