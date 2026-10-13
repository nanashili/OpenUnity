use engine_identifiers::ObjectId;
use engine_navigation::{FollowError, Path, PathError, PathFollower};

use crate::RuntimeError;

/// A caller-owned, resumable route bound to one GameObject and graph.
pub struct NavigationRoute<'graph> {
    pub(super) agent: ObjectId,
    pub(super) follower: PathFollower<'graph>,
}

impl NavigationRoute<'_> {
    /// Return the GameObject identity whose root Transform this route advances.
    pub fn agent(&self) -> ObjectId {
        self.agent
    }

    /// Return the selected shortest path retained by this route.
    pub fn path(&self) -> &Path {
        self.follower.path()
    }

    /// Return whether the route has reached its final graph node.
    pub fn is_complete(&self) -> bool {
        self.follower.is_complete()
    }
}

/// Failure reported while planning or advancing a runtime navigation route.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RuntimeNavigationError {
    /// Runtime object or Transform operation failed.
    Runtime(RuntimeError),
    /// The graph path could not be planned.
    Path(PathError),
    /// The graph path could not be followed within numeric bounds.
    Follow(FollowError),
    /// The graph exceeds the bounded runtime route limits.
    ResourceLimit,
    /// The agent's world position differs from the selected start node.
    StartPositionMismatch,
    /// The route agent is parented beneath another Transform.
    ParentTransformUnsupported,
    /// The route agent owns an attached physics body.
    PhysicsAttachmentUnsupported,
    /// The agent position changed since the previous committed route step.
    AgentPositionChanged,
}

impl From<RuntimeError> for RuntimeNavigationError {
    fn from(error: RuntimeError) -> Self {
        Self::Runtime(error)
    }
}

impl From<PathError> for RuntimeNavigationError {
    fn from(error: PathError) -> Self {
        Self::Path(error)
    }
}

impl From<FollowError> for RuntimeNavigationError {
    fn from(error: FollowError) -> Self {
        Self::Follow(error)
    }
}
