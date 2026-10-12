//! Independently designed, bounded authored-project scene import.
//! Only the reviewed authored text dialect is accepted; unsupported input fails.
#![forbid(unsafe_code)]

mod importer;
mod render_binding_owners;
mod yaml;

pub use engine_serialization::{
    FieldPlan, FieldValue, GameObjectPlan, MAX_DEPTH, MAX_FIELDS, MAX_INPUT_BYTES, MAX_OBJECTS,
    MAX_STRING_BYTES, SceneError, ScenePlan, ScriptPlan, TransformPlan, parse_plan,
};
pub use importer::import_project;
pub use render_binding_owners::validate_render_binding_owners;

use engine_serialization::{parse_json, validate_text as text};
type Result<T> = std::result::Result<T, SceneError>;
