//! Lossless, independently owned authoring state for the reviewed saved-scene dialect.

use std::fmt;

pub use engine_scene::FieldValue;

mod snapshot;
mod source;
mod views;

pub use snapshot::{ImportedDocument, ImportedEditorId, ImportedSnapshot};
pub use source::ImportedSource;
pub use views::{
    ImportedComponent, ImportedEntityEdit, ImportedEntityView, ImportedReference,
    ImportedReferenceKind, ImportedReferenceTarget,
};

/// A rejected import, archive or edit operation.
#[derive(Clone, Debug)]
pub struct ImportedError {
    /// Operation that failed.
    pub operation: &'static str,
    /// Owned explanation suitable for an editor diagnostic.
    pub message: String,
}
impl ImportedError {
    pub(crate) fn new(operation: &'static str, message: impl fmt::Display) -> Self {
        Self {
            operation,
            message: message.to_string(),
        }
    }
}
impl fmt::Display for ImportedError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.operation, self.message)
    }
}
impl std::error::Error for ImportedError {}
type Result<T> = std::result::Result<T, ImportedError>;
