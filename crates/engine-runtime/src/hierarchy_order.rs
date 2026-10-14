//! Parent-local sibling reordering admission and mutation.

use crate::{Runtime, RuntimeError};
use engine_object::ObjectId;

impl Runtime {
    /// Move a live Transform within its existing direct parent's child sequence.
    /// # Errors
    /// Rejects stale/root/malformed identities, unavailable scenes, and invalid indices.
    pub fn transform_set_sibling_index(
        &mut self,
        transform: ObjectId,
        index: i32,
    ) -> Result<(), RuntimeError> {
        self.validate_sibling_index_target(transform)?;

        let parent = self
            .hierarchy
            .validated_parent(transform)
            .map_err(crate::hierarchy::map_hierarchy_error)?
            .ok_or(RuntimeError::Unsupported)?;
        let children = self.validate_sibling_group(transform, parent)?;
        self.validate_sibling_scene(parent)?;

        let index = usize::try_from(index).map_err(|_| RuntimeError::InvalidSiblingIndex)?;
        if index >= children.len() {
            return Err(RuntimeError::InvalidSiblingIndex);
        }

        self.hierarchy
            .set_sibling_index(transform, index)
            .map_err(crate::hierarchy::map_hierarchy_error)
    }

    fn validate_sibling_index_target(&self, transform: ObjectId) -> Result<(), RuntimeError> {
        if self.scene_staging() {
            return Err(RuntimeError::StagingBusy);
        }
        if !self.is_alive(transform) {
            return Err(RuntimeError::InvalidObject);
        }
        self.transform(transform)?;
        Ok(())
    }

    fn validate_sibling_group(
        &self,
        transform: ObjectId,
        parent: ObjectId,
    ) -> Result<&[ObjectId], RuntimeError> {
        self.hierarchy
            .validate_parented_group(transform)
            .map_err(crate::hierarchy::map_hierarchy_error)?;
        self.validate_live_transform(parent)?;

        let children = self
            .hierarchy
            .children(parent)
            .filter(|children| !children.is_empty())
            .ok_or(RuntimeError::InvalidKind)?;
        let mut target_count = 0;
        for &child in children {
            self.validate_live_transform(child)?;
            if self.hierarchy.parent_of(child) != Some(parent) {
                return Err(RuntimeError::InvalidKind);
            }
            if !self.scene_hierarchy_owners_match(parent, child) {
                return Err(RuntimeError::InvalidKind);
            }
            target_count += usize::from(child == transform);
        }

        if target_count != 1 {
            return Err(RuntimeError::InvalidKind);
        }
        Ok(children)
    }

    fn validate_live_transform(&self, transform: ObjectId) -> Result<(), RuntimeError> {
        self.transform(transform)
            .map(|_| ())
            .map_err(|_| RuntimeError::InvalidKind)
    }

    fn validate_sibling_scene(&self, parent: ObjectId) -> Result<(), RuntimeError> {
        self.scene_hierarchy_admission(parent)
    }
}
