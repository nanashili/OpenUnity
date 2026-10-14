use super::SceneHierarchy;
use engine_identifiers::ObjectId;
use std::fmt;

/// The owner of a Transform that has no parent.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum RootOwner {
    /// A Transform created outside a published scene.
    Unowned,
    /// A Transform registered in the root group for this scene handle.
    Scene(ObjectId),
}

/// The destination membership for a parent change.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HierarchyTarget {
    /// Make the Transform a root owned by the given group.
    Root(RootOwner),
    /// Make the Transform a direct child of an existing Transform.
    Child(ObjectId),
}

/// A structural failure detected by the scene hierarchy graph.
#[derive(Debug, Eq, PartialEq)]
pub enum HierarchyError {
    /// A referenced Transform or root group is absent.
    MissingNode,
    /// A Transform already belongs to this graph.
    DuplicateNode,
    /// Stored parent, child, or root memberships disagree.
    MalformedTopology,
    /// A reparent request would make an ancestor its own descendant.
    Cycle,
    /// The parent-local reorder operation cannot reorder a root.
    RootReorderingUnsupported,
    /// A requested sibling index is outside its direct group.
    InvalidSiblingIndex {
        /// Requested zero-based sibling position.
        index: usize,
        /// Number of entries in the selected sibling group.
        count: usize,
    },
    /// A required collection could not reserve capacity.
    Allocation,
    /// A prepared graph transaction no longer matches the current graph.
    StalePreparation,
}

impl fmt::Display for HierarchyError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::MissingNode => "hierarchy node or root group does not exist",
            Self::DuplicateNode => "hierarchy node already exists",
            Self::MalformedTopology => "hierarchy memberships are inconsistent",
            Self::Cycle => "hierarchy operation would create a cycle",
            Self::RootReorderingUnsupported => "parent-local sibling reordering requires a parent",
            Self::InvalidSiblingIndex { .. } => "sibling index is outside the selected group",
            Self::Allocation => "hierarchy storage could not reserve capacity",
            Self::StalePreparation => "prepared hierarchy change is stale",
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for HierarchyError {}

/// Capacity reserved for adding one Transform to a selected group.
#[derive(Debug)]
pub struct PreparedRegistration {
    pub(super) target: HierarchyTarget,
    pub(super) base_revision: u64,
    pub(super) committed_revision: u64,
    pub(super) new_scene_roots: Option<Vec<ObjectId>>,
}

/// Fully built graph replacement for an unpublished scene publication.
#[derive(Debug)]
pub struct PreparedSceneMerge {
    pub(super) base_revision: u64,
    pub(super) candidate: SceneHierarchy,
}

/// Fully built replacement graph for a topology transaction.
#[derive(Debug)]
pub struct PreparedHierarchyChange {
    pub(super) base_revision: u64,
    pub(super) candidate: SceneHierarchy,
}
