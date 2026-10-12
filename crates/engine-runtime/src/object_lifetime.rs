//! Object allocation, mandatory Transform ownership, and synchronous removal.

use crate::{ObjectKind, Runtime, RuntimeError, TransformData};
use engine_object::{IdentityExhausted, ObjectId};

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
            ObjectKind::GameObject => self.gameobject_transform(id)?,
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
            self.objects.remove_object(data.owner)?;
            self.transforms
                .remove(&transform)
                .expect("validated Transform retirement");
        }
        Ok(())
    }
}
