//! Atomic detachment of a Transform's direct children.

use crate::{Runtime, RuntimeError, TransformData, math};
use engine_object::ObjectId;
use engine_scene::{HierarchyTarget, RootOwner};

struct DetachedPose {
    transform: ObjectId,
    position: math::Vector,
    rotation: math::Quaternion,
    scale: math::Vector,
}

impl Runtime {
    /// Move every direct child to the same root group while preserving its world pose.
    ///
    /// Descendant relationships below each moved child remain unchanged.
    /// # Errors
    /// Returns identity, scene, hierarchy, physics, numeric, or lifecycle errors.
    pub fn transform_detach_children(&mut self, parent: ObjectId) -> Result<(), RuntimeError> {
        self.validate_detach_parent(parent)?;
        let children = self.validated_detach_children(parent)?;
        if children.is_empty() {
            return Ok(());
        }

        let root_target = self.detached_root_target(parent);
        let poses = self.prepare_detached_poses(&children)?;
        self.validate_detached_pose_storage(&poses)?;
        let notifications = self.prepare_detach_children_notifications(parent, &children)?;
        let graph_change = self
            .hierarchy
            .prepare_reparent_batch(&children, root_target)
            .map_err(crate::hierarchy::map_hierarchy_error)?;

        self.hierarchy
            .commit_change(graph_change)
            .map_err(crate::hierarchy::map_hierarchy_error)?;
        self.commit_detached_poses(poses);
        self.commit_hierarchy_notifications(notifications);
        Ok(())
    }

    fn validate_detach_parent(&self, parent: ObjectId) -> Result<(), RuntimeError> {
        if self.scene_staging() {
            return Err(RuntimeError::StagingBusy);
        }
        if !self.is_alive(parent) {
            return Err(RuntimeError::InvalidObject);
        }

        self.transform(parent)?;
        self.scene_hierarchy_admission(parent)
    }

    fn validated_detach_children(&self, parent: ObjectId) -> Result<Vec<ObjectId>, RuntimeError> {
        let stored = self.transform_children(parent)?;
        let mut children = Vec::new();
        children
            .try_reserve(stored.len())
            .map_err(|_| RuntimeError::CountOverflow)?;
        children.extend_from_slice(stored);

        if let Some(&first_child) = children.first() {
            self.hierarchy
                .validate_parented_group(first_child)
                .map_err(crate::hierarchy::map_hierarchy_error)?;
        }

        for &child in &children {
            self.validate_detach_child(parent, child)?;
        }

        Ok(children)
    }

    fn validate_detach_child(&self, parent: ObjectId, child: ObjectId) -> Result<(), RuntimeError> {
        if !self.is_alive(child) || self.transform_parent_id(child)? != Some(parent) {
            return Err(RuntimeError::InvalidKind);
        }
        if !self.scene_hierarchy_owners_match(parent, child) {
            return Err(RuntimeError::InvalidKind);
        }

        self.scene_hierarchy_admission(child)?;
        self.physics_validate_hierarchy_change(child, None)
    }

    fn detached_root_target(&self, parent: ObjectId) -> HierarchyTarget {
        let owner = self
            .scene_hierarchy_owner(parent)
            .map_or(RootOwner::Unowned, RootOwner::Scene);
        HierarchyTarget::Root(owner)
    }

    fn prepare_detached_poses(
        &self,
        children: &[ObjectId],
    ) -> Result<Vec<DetachedPose>, RuntimeError> {
        let mut poses = Vec::new();
        poses
            .try_reserve(children.len())
            .map_err(|_| RuntimeError::CountOverflow)?;

        for &child in children {
            let data: TransformData = self.transform(child)?.clone();
            let (position, rotation, scale) = self.reparent_local_pose(child, None, &data)?;
            poses.push(DetachedPose {
                transform: child,
                position,
                rotation,
                scale,
            });
        }

        Ok(poses)
    }

    fn validate_detached_pose_storage(&self, poses: &[DetachedPose]) -> Result<(), RuntimeError> {
        if poses
            .iter()
            .any(|pose| !self.transforms.contains_key(&pose.transform))
        {
            return Err(RuntimeError::InvalidKind);
        }

        Ok(())
    }

    fn commit_detached_poses(&mut self, poses: Vec<DetachedPose>) {
        for pose in poses {
            let data = self
                .transforms
                .get_mut(&pose.transform)
                .expect("preflighted Transform storage remains present during detachment");
            data.position = pose.position;
            data.rotation = pose.rotation;
            data.scale = pose.scale;
        }
    }
}
