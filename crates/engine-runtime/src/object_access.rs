//! Runtime adapters for domain-owned names and component identity queries.

use crate::{ObjectKind, Runtime, RuntimeError};
use engine_object::ObjectId;

pub use engine_object::MAX_OBJECT_NAME_BYTES;

impl Runtime {
    /// Borrow a live object's owned name, resolving components to their owner.
    pub fn object_name(&self, id: ObjectId) -> Option<&str> {
        self.objects.name(id)
    }

    /// Replace a live object's name with an owned UTF-8 copy.
    pub fn set_object_name(&mut self, id: ObjectId, name: &str) -> bool {
        self.objects.set_name(id, name)
    }

    /// Return the reviewed native kind identifier.
    /// # Errors
    /// Returns an object error for stale or foreign identities.
    pub fn object_kind(&self, id: ObjectId) -> Result<i32, RuntimeError> {
        Ok(match self.objects.kind(id)? {
            ObjectKind::Plain => 1,
            ObjectKind::GameObject => 2,
            ObjectKind::Transform => 3,
            ObjectKind::Script => 4,
        })
    }

    /// Resolve a GameObject's mandatory Transform.
    /// # Errors
    /// Returns an object or kind error.
    pub fn gameobject_transform(&self, id: ObjectId) -> Result<ObjectId, RuntimeError> {
        self.objects.gameobject_transform(id).map_err(Into::into)
    }

    /// Resolve the owner of the supported Component kind.
    /// # Errors
    /// Returns an object or kind error.
    pub fn component_gameobject(&self, id: ObjectId) -> Result<ObjectId, RuntimeError> {
        self.objects.component_gameobject(id).map_err(Into::into)
    }
}
