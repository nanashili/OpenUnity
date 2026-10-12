//! Owned names and immutable object/component relationship queries.

use crate::object_store::ObjectRecord;
use crate::{ObjectError, ObjectId, ObjectKind, ObjectStore};

/// Maximum UTF-8 byte count of an owned object name.
pub const MAX_OBJECT_NAME_BYTES: usize = 1_048_576;

impl ObjectStore {
    fn record(&self, id: ObjectId) -> Result<&ObjectRecord, ObjectError> {
        self.records.get(&id).ok_or(ObjectError::InvalidObject)
    }

    /// Return the category of a live identity.
    /// # Errors
    /// Rejects absent, stale, or foreign identities.
    pub fn kind(&self, id: ObjectId) -> Result<ObjectKind, ObjectError> {
        Ok(match self.record(id)? {
            ObjectRecord::Plain { .. } => ObjectKind::Plain,
            ObjectRecord::GameObject { .. } => ObjectKind::GameObject,
            ObjectRecord::Transform { .. } => ObjectKind::Transform,
            ObjectRecord::Script { .. } => ObjectKind::Script,
        })
    }

    fn name_owner(&self, id: ObjectId) -> Option<ObjectId> {
        match self.records.get(&id)? {
            ObjectRecord::Transform { owner } | ObjectRecord::Script { owner } => Some(*owner),
            _ => Some(id),
        }
    }

    /// Borrow an owned name, resolving components to their GameObject.
    pub fn name(&self, id: ObjectId) -> Option<&str> {
        match self.records.get(&self.name_owner(id)?)? {
            ObjectRecord::Plain { name } | ObjectRecord::GameObject { name, .. } => Some(name),
            _ => None,
        }
    }

    /// Replace a live object's name without changing it on invalid input.
    pub fn set_name(&mut self, id: ObjectId, name: &str) -> bool {
        if name.len() > MAX_OBJECT_NAME_BYTES {
            return false;
        }
        let Some(owner) = self.name_owner(id) else {
            return false;
        };
        match self.records.get_mut(&owner) {
            Some(
                ObjectRecord::Plain { name: owned } | ObjectRecord::GameObject { name: owned, .. },
            ) => {
                *owned = name.to_owned();
                true
            }
            _ => false,
        }
    }

    /// Resolve a live GameObject's mandatory Transform.
    /// # Errors
    /// Rejects absent or wrong-kind identities.
    pub fn gameobject_transform(&self, id: ObjectId) -> Result<ObjectId, ObjectError> {
        match self.record(id)? {
            ObjectRecord::GameObject { transform, .. } => Ok(*transform),
            _ => Err(ObjectError::InvalidKind),
        }
    }

    /// Resolve a live component's GameObject.
    /// # Errors
    /// Rejects absent or wrong-kind identities.
    pub fn component_gameobject(&self, id: ObjectId) -> Result<ObjectId, ObjectError> {
        match self.record(id)? {
            ObjectRecord::Transform { owner } | ObjectRecord::Script { owner } => Ok(*owner),
            _ => Err(ObjectError::InvalidKind),
        }
    }

    /// Borrow committed script IDs in publication order, excluding reservations.
    /// # Errors
    /// Rejects absent or wrong-kind owners.
    pub fn owner_scripts(&self, owner: ObjectId) -> Result<&[ObjectId], ObjectError> {
        match self.record(owner)? {
            ObjectRecord::GameObject { scripts, .. } => Ok(scripts),
            _ => Err(ObjectError::InvalidKind),
        }
    }

    /// Read a GameObject's independent activity flag.
    /// # Errors
    /// Rejects absent or wrong-kind owners.
    pub fn active_self(&self, owner: ObjectId) -> Result<bool, ObjectError> {
        match self.record(owner)? {
            ObjectRecord::GameObject { active_self, .. } => Ok(*active_self),
            _ => Err(ObjectError::InvalidKind),
        }
    }

    /// Update a GameObject's independent activity flag.
    /// # Errors
    /// Rejects absent or wrong-kind owners without mutation.
    pub fn set_active_self(&mut self, owner: ObjectId, value: bool) -> Result<(), ObjectError> {
        match self
            .records
            .get_mut(&owner)
            .ok_or(ObjectError::InvalidObject)?
        {
            ObjectRecord::GameObject { active_self, .. } => {
                *active_self = value;
                Ok(())
            }
            _ => Err(ObjectError::InvalidKind),
        }
    }
}
