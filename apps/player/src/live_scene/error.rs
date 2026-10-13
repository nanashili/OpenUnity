use std::error::Error;
use std::fmt::{self, Debug, Display, Formatter};

#[derive(Debug)]
pub(crate) struct LiveSceneError(String);

impl LiveSceneError {
    pub(super) fn new(message: impl Into<String>) -> Self {
        Self(message.into())
    }

    pub(super) fn context(context: &str, error: impl Display) -> Self {
        Self::new(format!("{context}: {error}"))
    }

    pub(super) fn debug_context(context: &str, error: impl Debug) -> Self {
        Self::new(format!("{context}: {error:?}"))
    }
}

impl Display for LiveSceneError {
    fn fmt(&self, output: &mut Formatter<'_>) -> fmt::Result {
        output.write_str(&self.0)
    }
}

impl Error for LiveSceneError {}

impl From<crate::physics_scene::error::PhysicsSceneError> for LiveSceneError {
    fn from(error: crate::physics_scene::error::PhysicsSceneError) -> Self {
        Self::context("physics scene", error)
    }
}
