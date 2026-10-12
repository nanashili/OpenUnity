//! Explicit object retirement after the runtime's callback and graph phases.

use crate::object_store::ObjectRecord;
use crate::{ObjectError, ObjectId, ObjectKind, ObjectStore};
use std::collections::HashSet;

impl ObjectStore {
    /// Retire a plain object or component, or an owner with no published optional components.
    /// Mandatory Transforms are retired only with their owner.
    /// # Errors
    /// Rejects absent/wrong identities or owners with published components or reservations.
    pub fn remove_object(&mut self, id: ObjectId) -> Result<(), ObjectError> {
        match self.kind(id)? {
            ObjectKind::Plain => {
                self.records.remove(&id).expect("validated live object");
            }
            ObjectKind::Transform => return Err(ObjectError::InvalidKind),
            ObjectKind::Script => self.remove_script(id)?,
            ObjectKind::Rigidbody => self.remove_rigidbody(id)?,
            ObjectKind::GameObject => {
                let transform = self.gameobject_transform(id)?;
                match self.records.get(&id).ok_or(ObjectError::InvalidObject)? {
                    ObjectRecord::GameObject {
                        scripts,
                        reserved_scripts,
                        reserved_rigidbody,
                        components,
                        ..
                    } if scripts.is_empty()
                        && reserved_scripts.is_empty()
                        && components.is_empty()
                        && reserved_rigidbody.is_none() => {}
                    _ => return Err(ObjectError::ComponentState),
                }
                self.records.remove(&id).expect("validated live object");
                self.records
                    .remove(&transform)
                    .expect("mandatory live Transform");
            }
        }
        Ok(())
    }

    fn remove_script(&mut self, id: ObjectId) -> Result<(), ObjectError> {
        let owner = self.component_gameobject(id)?;
        match self
            .records
            .get_mut(&owner)
            .ok_or(ObjectError::InvalidObject)?
        {
            ObjectRecord::GameObject {
                scripts,
                reserved_scripts,
                components,
                ..
            } => {
                scripts.retain(|&component| component != id);
                reserved_scripts.retain(|&component| component != id);
                components.retain(|&component| component != id);
            }
            _ => return Err(ObjectError::InvalidKind),
        }
        self.records.remove(&id).expect("validated live script");
        Ok(())
    }

    fn remove_rigidbody(&mut self, id: ObjectId) -> Result<(), ObjectError> {
        let owner = self.component_gameobject(id)?;
        let ObjectRecord::GameObject {
            reserved_rigidbody,
            components,
            ..
        } = self
            .records
            .get_mut(&owner)
            .ok_or(ObjectError::InvalidObject)?
        else {
            return Err(ObjectError::InvalidKind);
        };
        if *reserved_rigidbody == Some(id) {
            *reserved_rigidbody = None;
        }
        components.retain(|&component| component != id);
        self.records.remove(&id).ok_or(ObjectError::InvalidObject)?;
        Ok(())
    }

    /// Release a complete set of ownership groups without executing callbacks.
    /// Stale IDs are permitted for scene cleanup after ordinary destruction.
    /// # Errors
    /// Rejects incomplete owner/component groups without retiring any object.
    pub fn release_objects(&mut self, ids: &[ObjectId]) -> Result<(), ObjectError> {
        self.prepare_release(ids)?.commit();
        Ok(())
    }

    /// Validate complete ownership groups before another subsystem starts cleanup.
    /// # Errors
    /// Rejects any live owner whose components are not all included, or the reverse.
    pub fn validate_release(&self, ids: &[ObjectId]) -> Result<(), ObjectError> {
        self.validate_release_set(&ids.iter().copied().collect())
    }

    pub(crate) fn validate_release_set(&self, ids: &HashSet<ObjectId>) -> Result<(), ObjectError> {
        for (&id, record) in &self.records {
            let valid = match record {
                ObjectRecord::Plain { .. } => true,
                ObjectRecord::GameObject { transform, .. } => {
                    !ids.contains(&id) || ids.contains(transform)
                }
                ObjectRecord::Transform { owner } => ids.contains(&id) == ids.contains(owner),
                ObjectRecord::Script { owner } | ObjectRecord::Rigidbody { owner } => {
                    ids.contains(&id) == ids.contains(owner)
                }
            };
            if !valid {
                return Err(ObjectError::ComponentState);
            }
        }
        Ok(())
    }
}
