//! Owned, versioned scene transfer records and bounded serialization validation.
#![forbid(unsafe_code)]

mod input;
mod json;
mod records;
mod validation;

pub use input::{
    MAX_DEPTH, MAX_FIELDS, MAX_INPUT_BYTES, MAX_OBJECTS, MAX_STRING_BYTES, SceneError,
    validate_text,
};
pub use json::parse as parse_json;
pub use records::{FieldPlan, FieldValue, GameObjectPlan, ScenePlan, ScriptPlan, TransformPlan};
pub use validation::parse_plan;

use input::{Result, validate_text as text};
