//! Hidden component reservation, publication and enumeration.

use super::state::Script;
use crate::{Runtime, RuntimeError};
use engine_object::ObjectId;

impl Runtime {
    /// Reserve a hidden script identity before managed construction.
    /// # Errors
    /// Rejects a foreign/wrong/pending owner, faulted context, or exhausted identities.
    pub fn component_reserve(
        &mut self,
        owner: ObjectId,
        order: i32,
    ) -> Result<ObjectId, RuntimeError> {
        self.owner_scripts(owner)?;
        self.lifecycle_ready()?;
        if self.owner_pending_destroy(owner)? {
            return Err(RuntimeError::Lifecycle);
        }
        let id = self.objects.reserve_script(owner)?;
        self.lifecycle.scripts.insert(
            id.to_raw(),
            Script {
                owner,
                order,
                committed: false,
                enabled: true,
                awakened: false,
                started: false,
                notified_enabled: false,
                in_flight: false,
                pending_destroy: false,
                notified_destroy: false,
            },
        );
        Ok(id)
    }

    /// Publish a successfully constructed reservation in owner insertion order.
    /// # Errors
    /// Rejects invalid/committed reservations, pending owners, and faulted dispatch.
    pub fn component_commit(&mut self, id: ObjectId) -> Result<(), RuntimeError> {
        let script = self.script(id)?;
        let owner = script.owner;
        if script.committed || self.owner_pending_destroy(owner)? {
            return Err(RuntimeError::Lifecycle);
        }
        self.lifecycle_ready()?;
        self.objects.publish_script(id)?;
        self.lifecycle
            .scripts
            .get_mut(&id.to_raw())
            .ok_or(RuntimeError::InvalidObject)?
            .committed = true;
        Ok(())
    }

    /// Consume an unpublished reservation after a constructor fails.
    /// # Errors
    /// Rejects absent, wrong-kind, or already committed identities.
    pub fn component_abort(&mut self, id: ObjectId) -> Result<(), RuntimeError> {
        if self.script(id)?.committed {
            return Err(RuntimeError::Lifecycle);
        }
        self.lifecycle.scripts.remove(&id.to_raw());
        self.objects.remove_object(id)?;
        Ok(())
    }

    /// Return mandatory Transform plus committed scripts.
    /// # Errors
    /// Rejects absent/wrong owners and unrepresentable counts.
    pub fn gameobject_component_count(&self, owner: ObjectId) -> Result<i32, RuntimeError> {
        i32::try_from(self.owner_scripts(owner)?.len() + 1).map_err(|_| RuntimeError::CountOverflow)
    }

    /// Resolve the Transform-first native component enumeration.
    /// # Errors
    /// Rejects absent/wrong owners or an invalid index.
    pub fn gameobject_component_at(
        &self,
        owner: ObjectId,
        index: i32,
    ) -> Result<ObjectId, RuntimeError> {
        let scripts = self.owner_scripts(owner)?;
        if index < 0 {
            return Err(RuntimeError::InvalidKind);
        }
        if index == 0 {
            self.gameobject_transform(owner)
        } else {
            usize::try_from(index - 1)
                .ok()
                .and_then(|index| scripts.get(index).copied())
                .ok_or(RuntimeError::InvalidKind)
        }
    }
}
