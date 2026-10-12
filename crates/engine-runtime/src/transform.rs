//! Transform storage access, pose properties, and affine point conversion.

use crate::{
    ObjectKind, Runtime, RuntimeError, TransformData,
    math::{self, Quaternion, Vector},
};
use engine_object::ObjectId;
use std::collections::HashMap;

pub(crate) mod authored_pose;
use authored_pose::TransformEdit;

impl Runtime {
    pub(crate) fn transform(&self, id: ObjectId) -> Result<&TransformData, RuntimeError> {
        if self.objects.kind(id)? != ObjectKind::Transform {
            return Err(RuntimeError::InvalidKind);
        }
        self.transforms.get(&id).ok_or(RuntimeError::InvalidObject)
    }

    pub(crate) fn transform_mut(
        &mut self,
        id: ObjectId,
    ) -> Result<&mut TransformData, RuntimeError> {
        if self.objects.kind(id)? != ObjectKind::Transform {
            return Err(RuntimeError::InvalidKind);
        }
        self.transforms
            .get_mut(&id)
            .ok_or(RuntimeError::InvalidObject)
    }

    /// Return a Transform's optional parent.
    /// # Errors
    /// Returns an object or kind error.
    pub fn transform_parent(&self, id: ObjectId) -> Result<Option<ObjectId>, RuntimeError> {
        Ok(self.transform(id)?.parent)
    }
    /// Return the number of ordered children.
    /// # Errors
    /// Returns an object, kind, or count overflow error.
    pub fn transform_child_count(&self, id: ObjectId) -> Result<i32, RuntimeError> {
        i32::try_from(self.transform(id)?.children.len()).map_err(|_| RuntimeError::CountOverflow)
    }
    /// Return an ordered child identity.
    /// # Errors
    /// Returns an object, kind, or index error.
    pub fn transform_child(&self, id: ObjectId, index: i32) -> Result<ObjectId, RuntimeError> {
        let data = self.transform(id)?;
        usize::try_from(index)
            .ok()
            .and_then(|index| data.children.get(index).copied())
            .ok_or(RuntimeError::InvalidKind)
    }
    pub(crate) fn world_rotation(&self, id: ObjectId) -> Result<Quaternion, RuntimeError> {
        let mut current = Some(id);
        let mut result = [0.0, 0.0, 0.0, 1.0];
        while let Some(id) = current {
            let data = self.transform(id)?;
            result = math::multiply(data.rotation, result)?;
            current = data.parent;
        }
        Ok(result)
    }
    /// Convert a point through the complete affine ancestor chain.
    /// # Errors
    /// Returns an object/kind, numeric, or singular inverse error.
    pub fn transform_point(
        &self,
        id: ObjectId,
        mut point: Vector,
        inverse: bool,
    ) -> Result<Vector, RuntimeError> {
        self.transform(id)?;
        math::finite(&point)?;
        if inverse {
            let mut chain = Vec::new();
            let mut current = Some(id);
            while let Some(id) = current {
                let data = self.transform(id)?;
                chain.push(id);
                current = data.parent;
            }
            for id in chain.into_iter().rev() {
                let data = self.transform(id)?;
                point = math::divide(
                    math::rotate(
                        math::conjugate(data.rotation),
                        math::subtract(point, data.position)?,
                    )?,
                    data.scale,
                )?;
            }
        } else {
            let mut current = Some(id);
            while let Some(id) = current {
                let data = self.transform(id)?;
                point = math::add(
                    data.position,
                    math::rotate(data.rotation, math::scale(point, data.scale)?)?,
                )?;
                current = data.parent;
            }
        }
        Ok(point)
    }
    /// Read one approved local/world pose property into an owned value.
    /// # Errors
    /// Returns object/kind/property or numeric errors.
    pub fn transform_read(&self, id: ObjectId, property: i32) -> Result<Vec<f32>, RuntimeError> {
        let data = self.transform(id)?;
        match property {
            1 => Ok(data.position.to_vec()),
            2 => Ok(data.rotation.to_vec()),
            3 => Ok(data.scale.to_vec()),
            4 => Ok(self.transform_point(id, [0.0; 3], false)?.to_vec()),
            5 => Ok(self.world_rotation(id)?.to_vec()),
            _ => Err(RuntimeError::InvalidKind),
        }
    }
    /// Atomically update an approved local/world pose property.
    /// # Errors
    /// Returns object/kind/property, numeric, or singular inverse errors.
    pub fn transform_write(
        &mut self,
        id: ObjectId,
        property: i32,
        values: &[f32],
    ) -> Result<(), RuntimeError> {
        let data = self.transform(id)?;
        math::finite(values)?;
        let parent = data.parent;
        match property {
            1 | 3 | 4 => {
                let value: Vector = values.try_into().map_err(|_| RuntimeError::InvalidKind)?;
                let value = if property == 4 {
                    if let Some(parent) = parent {
                        self.transform_point(parent, value, true)?
                    } else {
                        value
                    }
                } else {
                    value
                };

                if property == 3 {
                    self.physics_validate_transform_edit(id, TransformEdit::Scale)?;
                }

                let prepared_revision = if matches!(property, 1 | 4) {
                    self.prepare_authored_position_write(id)?
                } else {
                    None
                };

                let data = self.transform_mut(id)?;
                if property == 3 {
                    data.scale = value;
                } else {
                    data.position = value;
                }

                if let Some(revision) = prepared_revision {
                    self.commit_authored_position_write(id, revision);
                }
            }
            2 | 5 => {
                let value: Quaternion = values.try_into().map_err(|_| RuntimeError::InvalidKind)?;
                self.physics_validate_transform_edit(id, TransformEdit::Rotation)?;
                let mut value = math::normalize(value)?;
                if property == 5 {
                    if let Some(parent) = parent {
                        value =
                            math::multiply(math::conjugate(self.world_rotation(parent)?), value)?;
                    }
                }
                self.transform_mut(id)?.rotation = value;
            }
            _ => return Err(RuntimeError::InvalidKind),
        }
        Ok(())
    }
}

impl Runtime {
    /// Borrow the exact Transform selected for native physics publication.
    ///
    /// Runtime physics admission must validate the owning GameObject and body
    /// attachment before calling this helper.  Keeping that validation here
    /// prevents the post-backend publication path from performing a fallible
    /// lookup or silently targeting a different Transform.
    pub(crate) fn checked_physics_transform_target(
        transforms: &mut HashMap<ObjectId, TransformData>,
        transform_id: ObjectId,
        owner_id: ObjectId,
    ) -> Result<&mut TransformData, RuntimeError> {
        let target = transforms
            .get_mut(&transform_id)
            .ok_or(RuntimeError::InvalidObject)?;
        if target.owner != owner_id {
            return Err(RuntimeError::InvalidObject);
        }
        Ok(target)
    }
}
