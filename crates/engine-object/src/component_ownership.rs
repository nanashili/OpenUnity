//! Reserved script identities and ordered publication to their owner.

use crate::object_store::ObjectRecord;
use crate::{ObjectError, ObjectId, ObjectStore};
use engine_identifiers::allocate_next;

/// Holds the hidden Rigidbody reservation until backend admission succeeds.
pub struct PreparedRigidbodyReservation<'store> {
    store: Option<&'store mut ObjectStore>,
    owner: ObjectId,
    body: ObjectId,
}

impl PreparedRigidbodyReservation<'_> {
    /// Return the identity held by this reservation.
    pub fn body_id(&self) -> ObjectId {
        self.body
    }

    /// Keep the reservation and release the store borrow to the caller.
    pub fn commit(mut self) -> ObjectId {
        self.store.take();
        self.body
    }
}

impl Drop for PreparedRigidbodyReservation<'_> {
    fn drop(&mut self) {
        let Some(store) = self.store.take() else {
            return;
        };

        let removed = store
            .records
            .remove(&self.body)
            .expect("prepared Rigidbody reservation remains live");
        assert!(matches!(
            removed,
            ObjectRecord::Rigidbody { owner } if owner == self.owner
        ));

        let ObjectRecord::GameObject {
            reserved_rigidbody, ..
        } = store
            .records
            .get_mut(&self.owner)
            .expect("prepared Rigidbody owner remains live")
        else {
            unreachable!("prepared Rigidbody owner remains a GameObject");
        };
        assert_eq!(*reserved_rigidbody, Some(self.body));
        *reserved_rigidbody = None;
    }
}

impl ObjectStore {
    /// Reserve a component identity without publishing it in owner enumeration.
    /// # Errors
    /// Rejects absent/wrong owners or exhausted identities before mutation.
    pub fn reserve_script(&mut self, owner: ObjectId) -> Result<ObjectId, ObjectError> {
        self.prepare_component_capacity(owner, true)?;
        let id = allocate_next()?;
        let ObjectRecord::GameObject {
            reserved_scripts, ..
        } = self
            .records
            .get_mut(&owner)
            .ok_or(ObjectError::InvalidObject)?
        else {
            return Err(ObjectError::InvalidKind);
        };
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
                components,
                ..
            } => {
                let index = reserved_scripts
                    .iter()
                    .position(|&reserved| reserved == id)
                    .ok_or(ObjectError::ComponentState)?;
                reserved_scripts.remove(index);
                scripts.push(id);
                components.push(id);
                Ok(())
            }
            _ => Err(ObjectError::InvalidKind),
        }
    }

    /// Reserve a hidden Rigidbody identity for a GameObject.
    ///
    /// The object store owns the identity and reservation; physics owns the
    /// body state that is eventually associated with this identity.
    pub fn reserve_rigidbody(&mut self, owner: ObjectId) -> Result<ObjectId, ObjectError> {
        Ok(self.prepare_rigidbody_reservation(owner)?.commit())
    }

    /// Allocate a hidden Rigidbody while retaining a rollback borrow.
    pub fn prepare_rigidbody_reservation(
        &mut self,
        owner: ObjectId,
    ) -> Result<PreparedRigidbodyReservation<'_>, ObjectError> {
        self.prepare_component_capacity(owner, false)?;
        let id = allocate_next()?;
        let ObjectRecord::GameObject {
            reserved_rigidbody, ..
        } = self
            .records
            .get_mut(&owner)
            .expect("prepared owner remains live")
        else {
            unreachable!("prepared owner remains a GameObject");
        };
        *reserved_rigidbody = Some(id);
        self.records.insert(id, ObjectRecord::Rigidbody { owner });
        Ok(PreparedRigidbodyReservation {
            store: Some(self),
            owner,
            body: id,
        })
    }

    /// Publish one exact Rigidbody reservation in the full component catalog.
    pub fn publish_rigidbody(&mut self, id: ObjectId) -> Result<(), ObjectError> {
        self.prepare_rigidbody_publication(id)?.commit();
        Ok(())
    }

    pub(crate) fn prepare_component_capacity(
        &mut self,
        owner: ObjectId,
        script: bool,
    ) -> Result<(), ObjectError> {
        let Some(ObjectRecord::GameObject {
            reserved_rigidbody, ..
        }) = self.records.get(&owner)
        else {
            if self.records.contains_key(&owner) {
                return Err(ObjectError::InvalidKind);
            }
            return Err(ObjectError::InvalidObject);
        };
        if !script && reserved_rigidbody.is_some() {
            return Err(ObjectError::ComponentState);
        }
        self.records
            .try_reserve(1)
            .map_err(|_| ObjectError::CapacityExhausted)?;

        let ObjectRecord::GameObject {
            reserved_scripts,
            components,
            ..
        } = self
            .records
            .get_mut(&owner)
            .ok_or(ObjectError::InvalidObject)?
        else {
            return Err(ObjectError::InvalidKind);
        };
        if script {
            reserved_scripts
                .try_reserve(1)
                .map_err(|_| ObjectError::CapacityExhausted)?;
            components
                .try_reserve(1)
                .map_err(|_| ObjectError::CapacityExhausted)?;
        } else {
            components
                .try_reserve(1)
                .map_err(|_| ObjectError::CapacityExhausted)?;
        }
        Ok(())
    }
}
