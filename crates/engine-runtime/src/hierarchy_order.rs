//! Parent-local and scene-root sibling reordering admission and mutation.

use crate::{Runtime, RuntimeError};
use engine_object::ObjectId;
use engine_scene::RootOwner;

impl Runtime {
    /// Move a Transform to the end of its current ordered sibling group.
    /// # Errors
    /// Rejects stale or malformed identities and roots without a scene owner.
    pub fn transform_set_as_last_sibling(
        &mut self,
        transform: ObjectId,
    ) -> Result<(), RuntimeError> {
        let count = self.sibling_group_count(transform)?;
        let last_index = count.checked_sub(1).ok_or(RuntimeError::InvalidKind)?;
        let last_index = i32::try_from(last_index).map_err(|_| RuntimeError::CountOverflow)?;

        self.transform_set_sibling_index(transform, last_index)
    }

    fn sibling_group_count(&self, transform: ObjectId) -> Result<usize, RuntimeError> {
        self.validate_sibling_index_target(transform)?;

        match self
            .hierarchy
            .validated_parent(transform)
            .map_err(crate::hierarchy::map_hierarchy_error)?
        {
            Some(parent) => Ok(self.validate_sibling_group(transform, parent)?.len()),
            None => self.scene_root_sibling_count(transform),
        }
    }

    /// Move a live Transform within its parent's children or scene-root sequence.
    /// # Errors
    /// Rejects stale/malformed identities, unowned roots, unavailable scenes,
    /// and invalid indices.
    pub fn transform_set_sibling_index(
        &mut self,
        transform: ObjectId,
        index: i32,
    ) -> Result<(), RuntimeError> {
        self.validate_sibling_index_target(transform)?;

        let parent = self
            .hierarchy
            .validated_parent(transform)
            .map_err(crate::hierarchy::map_hierarchy_error)?;

        match parent {
            Some(parent) => self.set_parented_sibling_index(transform, parent, index),
            None => self.set_scene_root_sibling_index(transform, index),
        }
    }

    fn set_parented_sibling_index(
        &mut self,
        transform: ObjectId,
        parent: ObjectId,
        index: i32,
    ) -> Result<(), RuntimeError> {
        let children = self.validate_sibling_group(transform, parent)?;
        self.validate_sibling_scene(parent)?;
        let index = checked_sibling_index(index, children.len())?;
        let current_index = children
            .iter()
            .position(|child| *child == transform)
            .ok_or(RuntimeError::InvalidKind)?;
        if current_index == index {
            return Ok(());
        }

        let notifications = self.prepare_sibling_notifications(parent)?;
        self.hierarchy
            .set_sibling_index(transform, index)
            .map_err(crate::hierarchy::map_hierarchy_error)?;

        self.commit_hierarchy_notifications(notifications);
        Ok(())
    }

    fn set_scene_root_sibling_index(
        &mut self,
        transform: ObjectId,
        index: i32,
    ) -> Result<(), RuntimeError> {
        let scene = match self.hierarchy.root_owner(transform) {
            Some(RootOwner::Scene(scene)) => scene,
            Some(RootOwner::Unowned) => return Err(RuntimeError::Unsupported),
            None => return Err(RuntimeError::InvalidKind),
        };
        let roots = self
            .hierarchy
            .validate_scene_root_group(scene, transform)
            .map_err(crate::hierarchy::map_hierarchy_error)?;
        self.validate_scene_root_group(scene, roots)?;
        self.scene_hierarchy_admission(transform)?;

        let index = checked_sibling_index(index, roots.len())?;
        self.hierarchy
            .set_scene_root_sibling_index(scene, transform, index)
            .map_err(crate::hierarchy::map_hierarchy_error)
    }

    fn scene_root_sibling_count(&self, transform: ObjectId) -> Result<usize, RuntimeError> {
        let scene = match self.hierarchy.root_owner(transform) {
            Some(RootOwner::Scene(scene)) => scene,
            Some(RootOwner::Unowned) => return Err(RuntimeError::Unsupported),
            None => return Err(RuntimeError::InvalidKind),
        };

        let roots = self
            .hierarchy
            .validate_scene_root_group(scene, transform)
            .map_err(crate::hierarchy::map_hierarchy_error)?;
        self.validate_scene_root_group(scene, roots)?;

        Ok(roots.len())
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

    fn validate_scene_root_group(
        &self,
        scene: ObjectId,
        roots: &[ObjectId],
    ) -> Result<(), RuntimeError> {
        for &root in roots {
            self.validate_live_transform(root)?;
            if self.hierarchy.parent_of(root).is_some()
                || self.hierarchy.root_owner(root) != Some(RootOwner::Scene(scene))
                || self.scene_hierarchy_owner(root) != Some(scene)
            {
                return Err(RuntimeError::InvalidKind);
            }
        }

        Ok(())
    }
}

fn checked_sibling_index(index: i32, count: usize) -> Result<usize, RuntimeError> {
    let index = usize::try_from(index).map_err(|_| RuntimeError::InvalidSiblingIndex)?;
    if index >= count {
        return Err(RuntimeError::InvalidSiblingIndex);
    }

    Ok(index)
}
