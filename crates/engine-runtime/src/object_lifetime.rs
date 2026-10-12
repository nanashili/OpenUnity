//! Object allocation, mandatory Transform ownership, and synchronous removal.

use crate::{ObjectKind, Runtime, RuntimeError, TransformData};
use engine_object::{IdentityExhausted, ObjectId};
use std::collections::HashMap;

struct PreparedTransformCleanup {
    parent: Option<(ObjectId, Vec<ObjectId>)>,
    transforms: Vec<ObjectId>,
}

impl PreparedTransformCleanup {
    fn apply(self, storage: &mut HashMap<ObjectId, TransformData>) {
        let Self { parent, transforms } = self;
        for (id, data) in storage.iter_mut() {
            if let Some((parent_id, original_children)) = parent.as_ref()
                && parent_id == id
            {
                data.children.retain(|child| {
                    original_children.contains(child) && !transforms.contains(child)
                });
            } else {
                data.children.retain(|child| !transforms.contains(child));
            }
        }
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
    /// Returns [`IdentityExhausted`] without leaving a partial live pair.
    /// The bridge validates the name bound before calling this method.
    pub fn create_gameobject(&mut self, name: &str) -> Result<ObjectId, IdentityExhausted> {
        let (owner, transform) = self.objects.create_gameobject(name)?;
        self.transforms.insert(
            transform,
            TransformData {
                owner,
                parent: None,
                children: Vec::new(),
                position: [0.0; 3],
                rotation: [0.0, 0.0, 0.0, 1.0],
                scale: [1.0; 3],
            },
        );
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
        let parent = self.transform(root)?.parent;
        let parent_cleanup = parent
            .map(|parent_id| {
                let data = self.transform(parent_id)?;
                Ok::<_, RuntimeError>((parent_id, data.children.clone()))
            })
            .transpose()?;
        let mut transforms = Vec::new();
        let mut owners = Vec::new();
        while let Some(transform) = pending.pop() {
            let data = self.transform(transform)?.clone();
            pending.extend(data.children);
            transforms.push(transform);
            owners.push(data.owner);
        }
        let cleanup = PreparedTransformCleanup {
            parent: parent_cleanup,
            transforms,
        };
        self.physics_retire_objects(&owners)?;
        cleanup.apply(&mut self.transforms);
        Ok(())
    }
}
