//! Canonical parent, ordered-child, and scene-root graph storage.

mod graph;
mod registration;
mod removal;
mod reparent;
mod scene_detachment;
mod scene_merge;
mod sibling_order;
mod types;
mod unique;
mod validation;

pub use graph::SceneHierarchy;
pub use types::{
    HierarchyError, HierarchyTarget, PreparedHierarchyChange, PreparedRegistration,
    PreparedSceneMerge, RootOwner,
};
