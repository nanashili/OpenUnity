//! Independently specified scene authoring model (INFRA-EDITOR-001).

use std::fmt;

mod entity;
mod storage;

pub use entity::{Entity, EntityId, Primitive, Transform};
pub use storage::Scene;

/// Maximum UTF-8 bytes in a scene or entity name.
pub const MAX_NAME_BYTES: usize = 1_024;
/// Maximum accepted entities in one document.
pub const MAX_ENTITIES: usize = 100_000;
/// Maximum hierarchy depth, counting a root as depth one.
pub const MAX_HIERARCHY_DEPTH: usize = 256;

/// An authoring failure with the operation and a user-facing explanation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SceneError {
    operation: &'static str,
    detail: String,
}

impl SceneError {
    fn new(operation: &'static str, detail: impl Into<String>) -> Self {
        Self {
            operation,
            detail: detail.into(),
        }
    }

    /// The operation that failed.
    pub fn operation(&self) -> &'static str {
        self.operation
    }

    /// The reason the operation failed.
    pub fn detail(&self) -> &str {
        &self.detail
    }
}

impl fmt::Display for SceneError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}: {}", self.operation, self.detail)
    }
}

impl std::error::Error for SceneError {}
