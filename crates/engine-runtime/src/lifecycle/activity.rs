//! Native owner activity and committed behaviour enablement.

use crate::{Runtime, RuntimeError};
use engine_object::ObjectId;

impl Runtime {
    /// Read native self or hierarchy activity.
    /// # Errors
    /// Rejects an absent or wrong-kind owner.
    pub fn gameobject_active(
        &self,
        owner: ObjectId,
        hierarchy: bool,
    ) -> Result<bool, RuntimeError> {
        let mut current = Some(self.gameobject_transform(owner)?);
        while let Some(transform) = current {
            let data = self.transform(transform)?;
            if !self.objects.active_self(data.owner)? {
                return Ok(false);
            }
            current = if hierarchy {
                self.transform_parent_id(transform)?
            } else {
                None
            };
        }
        Ok(true)
    }

    /// Change activity; callback eligibility is reevaluated by the mutation pump.
    /// # Errors
    /// Rejects absent/wrong owners, pending destruction, faulted dispatch, or
    /// deactivation of a collision-body owner.
    pub fn gameobject_set_active(
        &mut self,
        owner: ObjectId,
        value: bool,
    ) -> Result<(), RuntimeError> {
        self.owner_scripts(owner)?;
        self.lifecycle_ready()?;
        if self.owner_pending_destroy(owner)? {
            return Err(RuntimeError::Lifecycle);
        }
        if !value && self.collision_scene.contains_owner(owner) {
            return Err(RuntimeError::PhysicsUnsupported);
        }
        self.objects.set_active_self(owner, value)?;
        Ok(())
    }

    /// Read the native enabled flag for a committed behaviour.
    /// # Errors
    /// Rejects absent/wrong or unpublished scripts.
    pub fn behaviour_enabled(&self, id: ObjectId) -> Result<bool, RuntimeError> {
        Ok(self.committed_script(id)?.enabled)
    }

    /// Change a committed script's enabled flag.
    /// # Errors
    /// Rejects absent/wrong/unpublished/pending scripts and faulted dispatch.
    pub fn behaviour_set_enabled(&mut self, id: ObjectId, value: bool) -> Result<(), RuntimeError> {
        if self.committed_script(id)?.pending_destroy {
            return Err(RuntimeError::Lifecycle);
        }
        self.lifecycle_ready()?;
        self.lifecycle
            .scripts
            .get_mut(&id.to_raw())
            .ok_or(RuntimeError::InvalidObject)?
            .enabled = value;
        Ok(())
    }

    /// Read effective native behaviour activity.
    /// # Errors
    /// Rejects absent/wrong or unpublished scripts.
    pub fn behaviour_active(&self, id: ObjectId) -> Result<bool, RuntimeError> {
        let script = self.committed_script(id)?;
        Ok(script.awakened
            && script.enabled
            && !script.pending_destroy
            && self.gameobject_active(script.owner, true)?)
    }
}
