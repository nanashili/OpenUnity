use std::error::Error;
use std::fmt::{self, Display, Formatter};

/// A route failure with the operation context needed by the CLI and receipt.
#[derive(Debug)]
pub(crate) struct PhysicsSceneError {
    message: String,
}

impl PhysicsSceneError {
    pub(crate) fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }

    pub(super) fn display(context: &str, error: impl Display) -> Self {
        Self::new(format!("{context}: {error}"))
    }

    pub(super) fn debug(context: &str, error: impl std::fmt::Debug) -> Self {
        Self::new(format!("{context}: {error:?}"))
    }
}

impl Display for PhysicsSceneError {
    fn fmt(&self, output: &mut Formatter<'_>) -> fmt::Result {
        output.write_str(&self.message)
    }
}

impl Error for PhysicsSceneError {}
