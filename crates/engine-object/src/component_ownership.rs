//! Reserved script identities and ordered publication to their owner.

use crate::object_store::ObjectRecord;
use crate::{ObjectError, ObjectId, ObjectStore};
use engine_identifiers::allocate_next;

impl ObjectStore {
    /// Reserve a component identity without publishing it in owner enumeration.
    /// # Errors
    /// Rejects absent/wrong owners or exhausted identities before mutation.
    pub fn reserve_script(&mut self, owner: ObjectId) -> Result<ObjectId, ObjectError> {
        let record = self
            .records
            .get_mut(&owner)
            .ok_or(ObjectError::InvalidObject)?;
        let ObjectRecord::GameObject {
            reserved_scripts, ..
        } = record
        else {
            return Err(ObjectError::InvalidKind);
        };
        let id = allocate_next()?;
        reserved_scripts.push(id);
        self.records.insert(id, ObjectRecord::Script { owner });
        Ok(id)
    }

    /// Publish an existing reservation exactly once, in commit order.
    /// # Errors
    /// Rejects absent/wrong identities or duplicate publication without mutation.
    pub fn publish_script(&mut self, id: ObjectId) -> Result<(), ObjectError> {
        let owner = match self.records.get(&id).ok_or(ObjectError::InvalidObject)? {
            ObjectRecord::Script { owner } => *owner,
            _ => return Err(ObjectError::InvalidKind),
        };
        match self
            .records
            .get_mut(&owner)
            .ok_or(ObjectError::InvalidObject)?
        {
            ObjectRecord::GameObject {
                scripts,
                reserved_scripts,
                ..
            } => {
                let index = reserved_scripts
                    .iter()
                    .position(|&reserved| reserved == id)
                    .ok_or(ObjectError::ComponentState)?;
                reserved_scripts.remove(index);
                scripts.push(id);
                Ok(())
            }
            _ => Err(ObjectError::InvalidKind),
        }
    }
}
