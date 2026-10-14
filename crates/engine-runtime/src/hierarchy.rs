//! Validated parent changes and world-pose preservation.

use crate::{
    Runtime, RuntimeError, TransformData,
    math::{self, Quaternion, Vector},
};
use engine_object::ObjectId;
use engine_scene::{HierarchyError, HierarchyTarget, RootOwner};

impl Runtime {
    fn uniform_ancestor_scale(&self, mut current: Option<ObjectId>) -> Result<f64, RuntimeError> {
        let mut scale = 1.0;
        while let Some(id) = current {
            let data = self.transform(id)?;
            if data.scale[0] <= 0.0
                || data.scale[0] != data.scale[1]
                || data.scale[0] != data.scale[2]
            {
                return Err(RuntimeError::Unsupported);
            }
            scale *= f64::from(data.scale[0]);
            if !scale.is_finite() || scale == 0.0 {
                return Err(RuntimeError::Numeric);
            }
            current = self.transform_parent_id(id)?;
        }
        Ok(scale)
    }
    /// Atomically reparent, preserving child order and optionally the supported world pose.
    /// # Errors
    /// Returns object/kind, cycle, numeric, or unsupported ancestor-scale errors.
    pub fn transform_set_parent(
        &mut self,
        id: ObjectId,
        parent: Option<ObjectId>,
        world_stays: bool,
    ) -> Result<(), RuntimeError> {
        // Validate every supplied identity before considering object kinds.
        if !self.is_alive(id) || parent.is_some_and(|parent| !self.is_alive(parent)) {
            return Err(RuntimeError::InvalidObject);
        }
        let data = self.transform(id)?.clone();
        let current_parent = self.transform_parent_id(id)?;
        if let Some(parent) = parent {
            self.transform(parent)?;
            self.scene_check_parent(id, parent)?;
        }
        let mut ancestor = parent;
        while let Some(current) = ancestor {
            if current == id {
                return Err(RuntimeError::Cycle);
            }
            ancestor = self.transform_parent_id(current)?;
        }
        if parent == current_parent {
            return Ok(());
        }

        // Same-parent requests return above before consulting physics.  For an
        // actual graph change, reject both moving an attached body root and
        // adding a child beneath one before computing or mutating any pose.
        self.physics_validate_hierarchy_change(id, parent)?;

        let (position, rotation, scale) = if world_stays {
            self.reparent_local_pose(id, parent, &data)?
        } else {
            (data.position, data.rotation, data.scale)
        };

        let notifications = self.prepare_reparent_notifications(current_parent, parent, id)?;
        let target = self.hierarchy_target(id, parent)?;
        self.hierarchy
            .reparent(id, target)
            .map_err(map_hierarchy_error)?;
        let data = self.transform_mut(id)?;
        data.position = position;
        data.rotation = rotation;
        data.scale = scale;
        self.commit_hierarchy_notifications(notifications);
        Ok(())
    }

    fn hierarchy_target(
        &self,
        transform: ObjectId,
        parent: Option<ObjectId>,
    ) -> Result<HierarchyTarget, RuntimeError> {
        if let Some(parent) = parent {
            return Ok(HierarchyTarget::Child(parent));
        }

        let target = self
            .scene_hierarchy_owner(transform)
            .map_or(RootOwner::Unowned, RootOwner::Scene);
        Ok(HierarchyTarget::Root(target))
    }

    fn reparent_local_pose(
        &self,
        id: ObjectId,
        parent: Option<ObjectId>,
        data: &TransformData,
    ) -> Result<(Vector, Quaternion, Vector), RuntimeError> {
        let old_scale = self.uniform_ancestor_scale(self.transform_parent_id(id)?)?;
        let new_scale = self.uniform_ancestor_scale(parent)?;
        let world_position = self.transform_point(id, [0.0; 3], false)?;
        let world_rotation = self.world_rotation(id)?;

        let position = if let Some(parent) = parent {
            self.transform_point(parent, world_position, true)?
        } else {
            world_position
        };
        let rotation = if let Some(parent) = parent {
            math::multiply(
                math::conjugate(self.world_rotation(parent)?),
                world_rotation,
            )?
        } else {
            world_rotation
        };
        let scale = math::vector(
            data.scale
                .map(|value| f64::from(value) * old_scale / new_scale),
        )?;

        Ok((position, rotation, scale))
    }
}

pub(crate) fn map_hierarchy_error(error: HierarchyError) -> RuntimeError {
    match error {
        HierarchyError::MissingNode | HierarchyError::DuplicateNode => RuntimeError::InvalidKind,
        HierarchyError::MalformedTopology => RuntimeError::InvalidKind,
        HierarchyError::Cycle => RuntimeError::Cycle,
        HierarchyError::RootReorderingUnsupported => RuntimeError::Unsupported,
        HierarchyError::InvalidSiblingIndex { .. } => RuntimeError::InvalidSiblingIndex,
        HierarchyError::Allocation => RuntimeError::CountOverflow,
        HierarchyError::StalePreparation => RuntimeError::InvalidKind,
    }
}
