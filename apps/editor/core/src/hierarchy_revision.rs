//! Opaque identity for one retained hierarchy-content snapshot.

use std::sync::Arc;

/// Opaque identity token used to invalidate in-memory hierarchy projections.
///
/// Cloning retains the same content identity. Successful content mutations
/// publish a fresh token; the token has no persisted representation.
#[derive(Clone, Debug)]
pub struct HierarchyRevision(Arc<()>);

impl HierarchyRevision {
    /// Create a distinct identity for newly validated hierarchy content.
    pub(crate) fn fresh() -> Self {
        Self(Arc::new(()))
    }
}

impl PartialEq for HierarchyRevision {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}

impl Eq for HierarchyRevision {}
