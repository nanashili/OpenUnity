use std::fmt;
use std::path::{Path, PathBuf};

/// A rejected scene document or failed scene file operation.
#[derive(Debug)]
pub struct SceneIoError {
    /// The operation that failed.
    pub operation: &'static str,
    /// The file involved, when the operation used the filesystem.
    pub path: Option<PathBuf>,
    /// The reason for the failure.
    pub message: String,
}

impl SceneIoError {
    pub(super) fn new(operation: &'static str, message: impl fmt::Display) -> Self {
        Self {
            operation,
            path: None,
            message: message.to_string(),
        }
    }

    pub(super) fn at(mut self, operation: &'static str, path: &Path) -> Self {
        self.operation = operation;
        self.path = Some(path.to_path_buf());
        self
    }
}

impl fmt::Display for SceneIoError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}", self.operation)?;
        if let Some(path) = &self.path {
            write!(formatter, " {}", path.display())?;
        }
        write!(formatter, ": {}", self.message)
    }
}

impl std::error::Error for SceneIoError {}
