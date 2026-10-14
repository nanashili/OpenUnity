//! Object allocation, mandatory Transform ownership, and synchronous removal.

use crate::{ObjectKind, Runtime, RuntimeError, TransformData};
use engine_object::{IdentityExhausted, ObjectId};
use std::collections::HashMap;

struct PreparedTransformCleanup {
    transforms: Vec<ObjectId>,
}

impl PreparedTransformCleanup {
    fn apply(self, storage: &mut HashMap<ObjectId, TransformData>) {
        let Self { transforms } = self;
        storage.retain(|id, _| !transforms.contains(id));
    }
}

impl Runtime {
    /// Allocate an independent plain object identity.
    /// # Errors
    /// Returns [`IdentityExhausted`] if the process identity space is exhausted.
    pub fn create_object(&mut self) -> Result<ObjectId, IdentityExhausted> {
        self.objects.create_object()
    }

    /// Atomically create a GameObject and its mandatory Transform.
    /// # Errors
    /// Returns a capacity, identity, or registration error without publishing a partial pair.
    /// The bridge validates the name bound before calling this method.
    pub fn create_gameobject(&mut self, name: &str) -> Result<ObjectId, RuntimeError> {
        let prepared_objects = self.objects.prepare_gameobject(name)?;
        self.transforms
            .try_reserve(1)
            .map_err(|_| RuntimeError::CountOverflow)?;
        let prepared_root = self
            .hierarchy
            .prepare_unowned_root_registration()
            .map_err(crate::hierarchy::map_hierarchy_error)?;
        let (owner, transform) = self.objects.create_prepared_gameobject(prepared_objects)?;

        self.transforms.insert(
            transform,
            TransformData {
                owner,
                position: [0.0; 3],
                rotation: [0.0, 0.0, 0.0, 1.0],
                scale: [1.0; 3],
            },
        );

        self.hierarchy.commit_registration(prepared_root, transform);
        Ok(owner)
    }

    /// Whether this runtime owns a live object with this identity.
    pub fn is_alive(&self, id: ObjectId) -> bool {
        self.objects.is_alive(id)
    }

    /// Destroy a plain object or GameObject subtree; mandatory Transforms reject destruction.
    pub fn destroy_object(&mut self, id: ObjectId) -> bool {
        self.destroy_typed_object(id).is_ok()
    }

    /// Destroy an owned object, preserving mandatory component ownership.
    /// # Errors
    /// Returns an object or kind error without mutation.
    pub fn destroy_typed_object(&mut self, id: ObjectId) -> Result<(), RuntimeError> {
        let root = match self.objects.kind(id)? {
            ObjectKind::Plain => {
                self.objects.remove_object(id)?;
                return Ok(());
            }
            ObjectKind::Transform => return Err(RuntimeError::InvalidKind),
            ObjectKind::Script => return Err(RuntimeError::Lifecycle),
            ObjectKind::Rigidbody => {
                self.physics_retire_objects(&[id])?;
                return Ok(());
            }
            ObjectKind::GameObject => self.gameobject_transform(id)?,
        };
        if self.subtree_has_scripts(root)? {
            return Err(RuntimeError::Lifecycle);
        }
        let mut pending = vec![root];
        let mut transforms = Vec::new();
        let mut owners = Vec::new();
        while let Some(transform) = pending.pop() {
            let data = self.transform(transform)?.clone();
            pending.extend(self.transform_children(transform)?.iter().copied());
            transforms.push(transform);
            owners.push(data.owner);
        }

        let prepared_hierarchy = self
            .hierarchy
            .prepare_removal(&transforms)
            .map_err(crate::hierarchy::map_hierarchy_error)?;
        let cleanup = PreparedTransformCleanup { transforms };

        self.physics_retire_objects(&owners)?;
        self.hierarchy
            .commit_change(prepared_hierarchy)
            .map_err(crate::hierarchy::map_hierarchy_error)?;
        cleanup.apply(&mut self.transforms);
        Ok(())
    }
}
