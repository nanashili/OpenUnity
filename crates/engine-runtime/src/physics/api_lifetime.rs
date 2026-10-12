//! Public Runtime ownership boundary for the collider-free Rigidbody slice.

use engine_identifiers::ObjectId;

use crate::transform::authored_pose::TransformEdit;
use crate::{ObjectKind, Runtime, RuntimeError};

use super::attachment::PhysicsAttachment;

impl Runtime {
    /// Reserve a hidden Rigidbody attached to a supported root GameObject.
    pub fn rigidbody_reserve(&mut self, owner: ObjectId) -> Result<ObjectId, RuntimeError> {
        let transform = self.validate_lifetime_admission(owner, true)?;
        let position = self.transform(transform)?.position;

        self.reserve_physics_body(owner, transform, position)
    }

    /// Publish a previously reserved Rigidbody after rechecking its owner.
    pub fn rigidbody_commit(&mut self, body: ObjectId) -> Result<(), RuntimeError> {
        let attachment = self.reserved_physics_attachment(body)?;
        if self.lifecycle_pending_owner(body) {
            return Err(RuntimeError::Lifecycle);
        }
        let transform = self.validate_lifetime_admission(attachment.owner_id, true)?;
        if transform != attachment.transform_id {
            return Err(RuntimeError::InvalidObject);
        }
        self.commit_physics_body(body)
    }

    /// Abort an unpublished Rigidbody reservation, including during cleanup.
    pub fn rigidbody_abort(&mut self, body: ObjectId) -> Result<(), RuntimeError> {
        if self.scene_staging() {
            return Err(RuntimeError::StagingBusy);
        }
        self.reserved_physics_attachment(body)?;
        self.abort_physics_body(body)
    }

    fn reserved_physics_attachment(
        &self,
        body: ObjectId,
    ) -> Result<PhysicsAttachment, RuntimeError> {
        let attachment = self
            .physics_attachment()
            .ok_or(RuntimeError::InvalidObject)?;
        if attachment.body_id != body {
            return Err(RuntimeError::InvalidObject);
        }
        if attachment.committed {
            return Err(RuntimeError::Lifecycle);
        }
        Ok(*attachment)
    }

    fn validate_lifetime_admission(
        &self,
        owner: ObjectId,
        require_healthy: bool,
    ) -> Result<ObjectId, RuntimeError> {
        if self.scene_staging() {
            return Err(RuntimeError::StagingBusy);
        }
        if require_healthy {
            self.lifecycle_ready()?;
        }
        if self.objects.kind(owner)? != ObjectKind::GameObject {
            return Err(RuntimeError::InvalidKind);
        }
        if self.owner_pending_destroy(owner)? {
            return Err(RuntimeError::Lifecycle);
        }
        if !self.gameobject_active(owner, true)? {
            return Err(RuntimeError::Lifecycle);
        }

        let transform = self.gameobject_transform(owner)?;
        self.validate_root_profile(transform)?;
        Ok(transform)
    }

    pub(super) fn validate_supported_owner_transform(
        &self,
        owner: ObjectId,
        transform: ObjectId,
    ) -> Result<(), RuntimeError> {
        if self.objects.kind(owner)? != ObjectKind::GameObject {
            return Err(RuntimeError::InvalidKind);
        }
        if self.gameobject_transform(owner)? != transform {
            return Err(RuntimeError::InvalidObject);
        }
        if self.owner_pending_destroy(owner)? {
            return Err(RuntimeError::Lifecycle);
        }
        if !self.gameobject_active(owner, true)? {
            return Err(RuntimeError::Lifecycle);
        }
        if let Some(attachment) = self.physics.attachment.filter(|attachment| {
            attachment.owner_id == owner && attachment.transform_id == transform
        }) {
            if self.physics_destruction_requested(attachment.body_id)? {
                return Err(RuntimeError::Lifecycle);
            }
        }
        self.validate_root_profile(transform)
    }

    pub(super) fn validate_physics_property_access(
        &self,
        body: ObjectId,
    ) -> Result<PhysicsAttachment, RuntimeError> {
        let attachment = self.committed_physics_attachment(body)?;
        self.validate_physics_state_access()?;
        self.validate_physics_owner(attachment.owner_id, body)?;
        self.validate_supported_owner_transform(attachment.owner_id, attachment.transform_id)?;
        Ok(attachment)
    }

    pub(super) fn validate_physics_step_owner(
        &self,
        owner: ObjectId,
        body: ObjectId,
    ) -> Result<(), RuntimeError> {
        self.validate_physics_state_access()?;
        let attachment = self.committed_physics_attachment(body)?;
        if attachment.owner_id != owner {
            return Err(RuntimeError::InvalidObject);
        }
        self.validate_physics_owner(owner, body)
    }

    pub(super) fn validate_physics_mode_access(&self) -> Result<(), RuntimeError> {
        self.validate_physics_state_access()
    }

    pub(super) fn validate_physics_sync_access(&self) -> Result<(), RuntimeError> {
        self.validate_physics_state_access()
    }

    fn validate_physics_state_access(&self) -> Result<(), RuntimeError> {
        self.lifecycle_ready()?;
        if self.scene_staging() {
            return Err(RuntimeError::StagingBusy);
        }
        Ok(())
    }

    fn committed_physics_attachment(
        &self,
        body: ObjectId,
    ) -> Result<PhysicsAttachment, RuntimeError> {
        let attachment = self
            .physics_attachment()
            .ok_or(RuntimeError::InvalidObject)?;
        if attachment.body_id != body {
            return Err(RuntimeError::InvalidObject);
        }
        if !attachment.committed {
            return Err(RuntimeError::Lifecycle);
        }
        Ok(*attachment)
    }

    fn validate_physics_owner(&self, owner: ObjectId, body: ObjectId) -> Result<(), RuntimeError> {
        if self.physics_destruction_requested(body)? || self.owner_pending_destroy(owner)? {
            return Err(RuntimeError::Lifecycle);
        }
        if !self.gameobject_active(owner, true)? {
            return Err(RuntimeError::Lifecycle);
        }
        Ok(())
    }

    pub(crate) fn physics_validate_transform_edit(
        &self,
        transform_id: ObjectId,
        edit: TransformEdit,
    ) -> Result<(), RuntimeError> {
        let Some(attachment) = self.physics.attachment else {
            return Ok(());
        };
        if attachment.transform_id == transform_id {
            match edit {
                TransformEdit::Rotation | TransformEdit::Scale => {
                    return Err(RuntimeError::PhysicsUnsupported);
                }
            }
        }
        Ok(())
    }

    pub(crate) fn physics_validate_hierarchy_change(
        &self,
        child_transform: ObjectId,
        new_parent: Option<ObjectId>,
    ) -> Result<(), RuntimeError> {
        let Some(attachment) = self.physics.attachment else {
            return Ok(());
        };
        if child_transform == attachment.transform_id || new_parent == Some(attachment.transform_id)
        {
            return Err(RuntimeError::PhysicsUnsupported);
        }
        Ok(())
    }

    pub(crate) fn validate_root_profile(&self, transform: ObjectId) -> Result<(), RuntimeError> {
        let data = self.transform(transform)?;
        if data.parent.is_some()
            || !data.children.is_empty()
            || data.scale != [1.0; 3]
            || data.rotation != [0.0, 0.0, 0.0, 1.0]
        {
            return Err(RuntimeError::PhysicsUnsupported);
        }
        Ok(())
    }
}
