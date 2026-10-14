use super::{HierarchyError, HierarchyTarget, PreparedHierarchyChange, SceneHierarchy};
use engine_identifiers::ObjectId;
use std::collections::HashSet;

impl SceneHierarchy {
    /// Prepare one atomic move of several Transform roots to the same parent group.
    ///
    /// The supplied order is retained in the destination group. Duplicate
    /// identities are rejected so one Transform cannot be moved twice.
    /// # Errors
    /// Rejects duplicate, missing, malformed, cross-owner, and allocation cases.
    pub fn prepare_reparent_batch(
        &self,
        transforms: &[ObjectId],
        target: HierarchyTarget,
    ) -> Result<PreparedHierarchyChange, HierarchyError> {
        self.validate_all()?;
        let mut unique = HashSet::new();
        unique
            .try_reserve(transforms.len())
            .map_err(|_| HierarchyError::Allocation)?;

        for &transform in transforms {
            if !unique.insert(transform) {
                return Err(HierarchyError::DuplicateNode);
            }
        }

        let mut candidate = self.try_clone_graph()?;
        for &transform in transforms {
            candidate.reparent(transform, target)?;
        }
        candidate.validate_all()?;

        Ok(PreparedHierarchyChange {
            base_revision: self.revision,
            candidate,
        })
    }
}
