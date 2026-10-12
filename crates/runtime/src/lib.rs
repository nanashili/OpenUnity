//! Independent runtime ownership and reviewed GameObject/Transform subset.
#![forbid(unsafe_code)]

mod lifecycle;
mod math;
mod scene;
mod timing;
pub use lifecycle::LifecycleEvent;
use math::{Quaternion, Vector};
use openunity_object_model::{IdentityExhausted, ObjectId, Registry};
pub use timing::{TimeSnapshot, validate_time_configuration, validate_time_property};

/// Maximum UTF-8 byte count of an owned object name.
pub const MAX_OBJECT_NAME_BYTES: usize = 1_048_576;

/// Independent typed-extension failures; these are not Unity native statuses.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RuntimeError {
    /// Identity is stale, foreign, or absent.
    InvalidObject,
    /// Object kind, index, or property is unsupported.
    InvalidKind,
    /// A numeric input or result is invalid.
    Numeric,
    /// A parent assignment would form a cycle.
    Cycle,
    /// A singular inverse or unsupported reparent was requested.
    Unsupported,
    /// A collection size cannot be represented by the ABI.
    CountOverflow,
    /// Invalid attachment, lifecycle phase, or callback acknowledgement.
    Lifecycle,
    /// Managed gameplay dispatch failed and cannot continue.
    DispatchFaulted,
    /// Process identities or per-context callback tokens are exhausted.
    IdentityExhausted,
    /// Invalid, stale, or foreign scene handle.
    InvalidScene,
    /// Invalid or unsupported scene plan.
    InvalidScenePlan,
    /// Staging, owner thread, or terminal operation violation.
    StagingBusy,
}

#[derive(Debug)]
struct ObjectRecord {
    name: String,
    kind: ObjectKind,
}
#[derive(Debug)]
enum ObjectKind {
    Plain,
    GameObject {
        transform: Option<ObjectId>,
        active_self: bool,
        scripts: Vec<ObjectId>,
    },
    Transform(TransformData),
    Script {
        owner: ObjectId,
    },
}
#[derive(Clone, Debug)]
struct TransformData {
    owner: ObjectId,
    parent: Option<ObjectId>,
    children: Vec<ObjectId>,
    position: Vector,
    rotation: Quaternion,
    scale: Vector,
}

/// Owns the identities of one engine context. Dropping it releases its objects.
#[derive(Debug, Default)]
pub struct Runtime {
    objects: Registry<ObjectRecord>,
    lifecycle: lifecycle::Lifecycle,
    scenes: scene::Scenes,
    clock: timing::Clock,
}

impl Runtime {
    /// Allocate an independent plain object identity.
    /// # Errors
    /// Returns [`IdentityExhausted`] if the process identity space is exhausted.
    pub fn create_object(&mut self) -> Result<ObjectId, IdentityExhausted> {
        self.objects.insert(ObjectRecord {
            name: String::new(),
            kind: ObjectKind::Plain,
        })
    }

    /// Atomically create a GameObject and its mandatory Transform.
    /// # Errors
    /// Returns [`IdentityExhausted`] without leaving a partial live pair.
    /// The bridge validates the name bound before calling this method.
    pub fn create_gameobject(&mut self, name: &str) -> Result<ObjectId, IdentityExhausted> {
        let owner = self.objects.insert(ObjectRecord {
            name: name.to_owned(),
            kind: ObjectKind::GameObject {
                transform: None,
                active_self: true,
                scripts: Vec::new(),
            },
        })?;
        let transform = match self.objects.insert(ObjectRecord {
            name: String::new(),
            kind: ObjectKind::Transform(TransformData {
                owner,
                parent: None,
                children: Vec::new(),
                position: [0.0; 3],
                rotation: [0.0, 0.0, 0.0, 1.0],
                scale: [1.0; 3],
            }),
        }) {
            Ok(id) => id,
            Err(error) => {
                self.objects.remove(owner);
                return Err(error);
            }
        };
        if let Some(record) = self.objects.get_mut(owner) {
            if let ObjectKind::GameObject {
                transform: slot, ..
            } = &mut record.kind
            {
                *slot = Some(transform);
            }
        }
        Ok(owner)
    }

    /// Whether this runtime owns a live object with this identity.
    pub fn is_alive(&self, id: ObjectId) -> bool {
        self.objects.get(id).is_some()
    }

    /// Destroy a plain object or GameObject subtree; mandatory Transforms reject destruction.
    pub fn destroy_object(&mut self, id: ObjectId) -> bool {
        self.destroy_typed_object(id).is_ok()
    }

    /// Destroy an owned object, preserving mandatory component ownership.
    /// # Errors
    /// Returns an object or kind error without mutation.
    pub fn destroy_typed_object(&mut self, id: ObjectId) -> Result<(), RuntimeError> {
        let record = self.objects.get(id).ok_or(RuntimeError::InvalidObject)?;
        let root = match record.kind {
            ObjectKind::Plain => {
                self.objects.remove(id);
                return Ok(());
            }
            ObjectKind::Transform(_) => return Err(RuntimeError::InvalidKind),
            ObjectKind::Script { .. } => return Err(RuntimeError::Lifecycle),
            ObjectKind::GameObject { transform, .. } => {
                transform.ok_or(RuntimeError::InvalidKind)?
            }
        };
        if self.subtree_has_scripts(root)? {
            return Err(RuntimeError::Lifecycle);
        }
        let parent = self.transform(root)?.parent;
        if let Some(parent) = parent {
            self.transform_mut(parent)?
                .children
                .retain(|child| *child != root);
        }
        let mut pending = vec![root];
        while let Some(transform) = pending.pop() {
            let data = self.transform(transform)?.clone();
            pending.extend(data.children);
            self.objects.remove(data.owner);
            self.objects.remove(transform);
        }
        Ok(())
    }

    fn name_owner(&self, id: ObjectId) -> Option<ObjectId> {
        match &self.objects.get(id)?.kind {
            ObjectKind::Transform(data) => Some(data.owner),
            ObjectKind::Script { owner } => Some(*owner),
            _ => Some(id),
        }
    }
    /// Borrow a live object's owned name, resolving Transform names to their owner.
    pub fn object_name(&self, id: ObjectId) -> Option<&str> {
        self.objects
            .get(self.name_owner(id)?)
            .map(|record| record.name.as_str())
    }
    /// Replace a live object's name with an owned UTF-8 copy.
    pub fn set_object_name(&mut self, id: ObjectId, name: &str) -> bool {
        if name.len() > MAX_OBJECT_NAME_BYTES {
            return false;
        }
        let Some(owner) = self.name_owner(id) else {
            return false;
        };
        let Some(record) = self.objects.get_mut(owner) else {
            return false;
        };
        record.name = name.to_owned();
        true
    }
    /// Return the reviewed native kind identifier.
    /// # Errors
    /// Returns an object error for stale or foreign identities.
    pub fn object_kind(&self, id: ObjectId) -> Result<i32, RuntimeError> {
        Ok(
            match self
                .objects
                .get(id)
                .ok_or(RuntimeError::InvalidObject)?
                .kind
            {
                ObjectKind::Plain => 1,
                ObjectKind::GameObject { .. } => 2,
                ObjectKind::Transform(_) => 3,
                ObjectKind::Script { .. } => 4,
            },
        )
    }
    /// Resolve a GameObject's mandatory Transform.
    /// # Errors
    /// Returns an object or kind error.
    pub fn gameobject_transform(&self, id: ObjectId) -> Result<ObjectId, RuntimeError> {
        match self
            .objects
            .get(id)
            .ok_or(RuntimeError::InvalidObject)?
            .kind
        {
            ObjectKind::GameObject {
                transform: Some(transform),
                ..
            } => Ok(transform),
            _ => Err(RuntimeError::InvalidKind),
        }
    }
    /// Resolve the owner of the supported Component kind.
    /// # Errors
    /// Returns an object or kind error.
    pub fn component_gameobject(&self, id: ObjectId) -> Result<ObjectId, RuntimeError> {
        match &self
            .objects
            .get(id)
            .ok_or(RuntimeError::InvalidObject)?
            .kind
        {
            ObjectKind::Transform(data) => Ok(data.owner),
            ObjectKind::Script { owner } => Ok(*owner),
            _ => Err(RuntimeError::InvalidKind),
        }
    }
    fn transform(&self, id: ObjectId) -> Result<&TransformData, RuntimeError> {
        match &self
            .objects
            .get(id)
            .ok_or(RuntimeError::InvalidObject)?
            .kind
        {
            ObjectKind::Transform(data) => Ok(data),
            _ => Err(RuntimeError::InvalidKind),
        }
    }
    fn transform_mut(&mut self, id: ObjectId) -> Result<&mut TransformData, RuntimeError> {
        match &mut self
            .objects
            .get_mut(id)
            .ok_or(RuntimeError::InvalidObject)?
            .kind
        {
            ObjectKind::Transform(data) => Ok(data),
            _ => Err(RuntimeError::InvalidKind),
        }
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
    fn world_rotation(&self, id: ObjectId) -> Result<Quaternion, RuntimeError> {
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
                let data = self.transform_mut(id)?;
                if property == 3 {
                    data.scale = value;
                } else {
                    data.position = value;
                }
            }
            2 | 5 => {
                let value: Quaternion = values.try_into().map_err(|_| RuntimeError::InvalidKind)?;
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
            current = data.parent;
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
        if let Some(parent) = parent {
            self.transform(parent)?;
            self.scene_check_parent(id, parent)?;
        }
        let mut ancestor = parent;
        while let Some(current) = ancestor {
            if current == id {
                return Err(RuntimeError::Cycle);
            }
            ancestor = self.transform(current)?.parent;
        }
        if parent == data.parent {
            return Ok(());
        }
        let (position, rotation, scale) = if world_stays {
            let old_scale = self.uniform_ancestor_scale(data.parent)?;
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
            (position, rotation, scale)
        } else {
            (data.position, data.rotation, data.scale)
        };
        if let Some(old) = data.parent {
            self.transform_mut(old)?
                .children
                .retain(|child| *child != id);
        }
        if let Some(parent) = parent {
            self.transform_mut(parent)?.children.push(id);
        }
        self.scene_update_roots(id, data.parent, parent);
        let data = self.transform_mut(id)?;
        data.parent = parent;
        data.position = position;
        data.rotation = rotation;
        data.scale = scale;
        Ok(())
    }
}
