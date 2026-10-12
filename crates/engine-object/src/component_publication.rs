//! Borrow-bound publication and retirement transactions for component owners.

use crate::object_store::ObjectRecord;
use crate::{ObjectError, ObjectId, ObjectStore};
use std::collections::HashSet;

/// Holds exclusive store access between Rigidbody validation and publication.
pub struct PreparedRigidbodyPublication<'store> {
    store: &'store mut ObjectStore,
    owner: ObjectId,
    body: ObjectId,
}

impl PreparedRigidbodyPublication<'_> {
    /// Publish the validated reservation without another fallible operation.
    pub fn commit(self) {
        let Self { store, owner, body } = self;
        let ObjectRecord::GameObject {
            reserved_rigidbody,
            components,
            ..
        } = store
            .records
            .get_mut(&owner)
            .expect("prepared owner remains live")
        else {
            unreachable!("prepared owner has stable GameObject kind");
        };
        debug_assert_eq!(*reserved_rigidbody, Some(body));
        *reserved_rigidbody = None;
        components.push(body);
    }
}

/// Holds exclusive store access for a complete ownership release.
pub struct PreparedObjectRelease<'store> {
    store: &'store mut ObjectStore,
    removal: HashSet<ObjectId>,
}

impl PreparedObjectRelease<'_> {
    /// Remove all validated identities without allocation or lookup failure.
    pub fn commit(self) {
        let Self { store, removal } = self;
        store.records.retain(|id, _| !removal.contains(id));
    }
}

/// Holds exclusive store access for ordinary direct or owner retirement.
pub struct PreparedObjectRetirement<'store> {
    store: &'store mut ObjectStore,
    removal: HashSet<ObjectId>,
}

impl PreparedObjectRetirement<'_> {
    /// Apply validated removal and prune surviving owner catalogs.
    pub fn commit(self) {
        let Self { store, removal } = self;
        store.records.retain(|id, _| !removal.contains(id));
        for record in store.records.values_mut() {
            if let ObjectRecord::GameObject {
                scripts,
                components,
                reserved_scripts,
                reserved_rigidbody,
                ..
            } = record
            {
                scripts.retain(|id| !removal.contains(id));
                components.retain(|id| !removal.contains(id));
                reserved_scripts.retain(|id| !removal.contains(id));
                if reserved_rigidbody.is_some_and(|id| removal.contains(&id)) {
                    *reserved_rigidbody = None;
                }
            }
        }
    }
}

impl ObjectStore {
    /// Validate and hold a reserved Rigidbody until an external owner commits it.
    pub fn prepare_rigidbody_publication(
        &mut self,
        body: ObjectId,
    ) -> Result<PreparedRigidbodyPublication<'_>, ObjectError> {
        let owner = match self.records.get(&body).ok_or(ObjectError::InvalidObject)? {
            ObjectRecord::Rigidbody { owner } => *owner,
            _ => return Err(ObjectError::InvalidKind),
        };
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
        if *reserved_rigidbody != Some(body) {
            return Err(ObjectError::ComponentState);
        }
        components
            .try_reserve(1)
            .map_err(|_| ObjectError::CapacityExhausted)?;
        Ok(PreparedRigidbodyPublication {
            store: self,
            owner,
            body,
        })
    }

    /// Validate a complete release and retain the store borrow until commit.
    pub fn prepare_release(
        &mut self,
        ids: &[ObjectId],
    ) -> Result<PreparedObjectRelease<'_>, ObjectError> {
        let mut removal = HashSet::new();
        removal
            .try_reserve(ids.len())
            .map_err(|_| ObjectError::CapacityExhausted)?;
        removal.extend(ids.iter().copied());
        self.validate_release_set(&removal)?;
        Ok(PreparedObjectRelease {
            store: self,
            removal,
        })
    }

    /// Validate direct optional-component or complete-owner retirement.
    pub fn prepare_retirement(
        &mut self,
        ids: &[ObjectId],
    ) -> Result<PreparedObjectRetirement<'_>, ObjectError> {
        let mut removal = HashSet::new();
        removal
            .try_reserve(ids.len())
            .map_err(|_| ObjectError::CapacityExhausted)?;
        for &id in ids {
            let Some(record) = self.records.get(&id) else {
                continue;
            };
            match record {
                ObjectRecord::Plain { .. }
                | ObjectRecord::Script { .. }
                | ObjectRecord::Rigidbody { .. } => {
                    removal.insert(id);
                }
                ObjectRecord::Transform { .. } => return Err(ObjectError::InvalidKind),
                ObjectRecord::GameObject {
                    transform,
                    scripts,
                    components,
                    reserved_scripts,
                    reserved_rigidbody,
                    ..
                } => {
                    removal.insert(id);
                    removal.insert(*transform);
                    removal.extend(scripts.iter().copied());
                    removal.extend(components.iter().copied());
                    removal.extend(reserved_scripts.iter().copied());
                    if let Some(body) = reserved_rigidbody {
                        removal.insert(*body);
                    }
                }
            }
        }
        self.validate_removal_set(&removal)?;
        Ok(PreparedObjectRetirement {
            store: self,
            removal,
        })
    }

    fn validate_removal_set(&self, removal: &HashSet<ObjectId>) -> Result<(), ObjectError> {
        for id in removal {
            let Some(record) = self.records.get(id) else {
                continue;
            };
            if let ObjectRecord::Transform { owner } = record {
                if !removal.contains(owner) {
                    return Err(ObjectError::InvalidKind);
                }
            }
        }
        Ok(())
    }
}
