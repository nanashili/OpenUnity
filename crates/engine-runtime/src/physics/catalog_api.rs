//! Runtime-facing access to the complete Transform-first component catalog.

use crate::{Runtime, RuntimeError};
use engine_object::ObjectId;

impl Runtime {
    /// Return the complete committed catalog length for one GameObject.
    ///
    /// Transform occupies index zero. Hidden reservations are excluded by the
    /// ObjectStore catalog owner, so this adapter cannot publish them early.
    /// # Errors
    /// Returns an object or kind error for stale and non-owner identities.
    pub fn physics_component_count(&self, owner: ObjectId) -> Result<usize, RuntimeError> {
        self.objects
            .owner_component_count(owner)
            .map_err(Into::into)
    }

    /// Resolve one committed entry from the complete Transform-first catalog.
    /// # Errors
    /// Returns an object or kind error for stale, non-owner, or out-of-range inputs.
    pub fn physics_component_at(
        &self,
        owner: ObjectId,
        index: usize,
    ) -> Result<ObjectId, RuntimeError> {
        self.objects
            .owner_component_at(owner, index)
            .map_err(Into::into)
    }
}
