//! Runtime ownership of the one native motion body and its Transform link.
//!
//! This module is a composition candidate.  Object identity and publication
//! belong to `engine_object`; body state and reservation revisions belong to
//! `engine_physics_3d`.  The attachment stores only the relationship between
//! those owners and the property metadata needed for Transform synchronization.

use engine_identifiers::ObjectId;
use engine_math::Vector;
use engine_physics_3d::MotionWorld;
use engine_physics_api::{BodyId, BodyState, SimulationMode};

use crate::{Runtime, RuntimeError};

use super::body_properties::BodyProperties;

/// Private relationship between a Rigidbody identity and its owner Transform.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct PhysicsAttachment {
    pub(super) body_id: ObjectId,
    pub(super) owner_id: ObjectId,
    pub(super) transform_id: ObjectId,
    pub(super) committed: bool,
    pub(super) properties: BodyProperties,
}

impl PhysicsAttachment {
    pub(super) fn reserved(
        body_id: ObjectId,
        owner_id: ObjectId,
        transform_id: ObjectId,
        properties: BodyProperties,
    ) -> Self {
        Self {
            body_id,
            owner_id,
            transform_id,
            committed: false,
            properties,
        }
    }

    pub(super) fn is_dirty(self) -> bool {
        self.properties.is_position_dirty()
    }
}

/// Runtime-owned physics state added to `Runtime` by the context patch.
#[derive(Debug, Default)]
pub(crate) struct PhysicsState {
    pub(super) world: MotionWorld,
    pub(super) mode: SimulationMode,
    pub(super) attachment: Option<PhysicsAttachment>,
}

impl Runtime {
    /// Reserve a hidden Rigidbody and the matching backend body atomically.
    ///
    /// Callers perform lifecycle, staging, activity and profile admission
    /// before this operation.  The object reservation is deliberately made
    /// before backend reservation so the backend receives an owned identity.
    /// If backend reservation fails, the exact object reservation is removed
    /// synchronously and no attachment escapes the failed operation.
    pub(super) fn reserve_physics_body(
        &mut self,
        owner_id: ObjectId,
        transform_id: ObjectId,
        position: Vector,
    ) -> Result<ObjectId, RuntimeError> {
        if self.physics.attachment.is_some() {
            return Err(RuntimeError::PhysicsUnsupported);
        }

        let reservation = self.objects.prepare_rigidbody_reservation(owner_id)?;
        let body_id = reservation.body_id();

        self.physics
            .world
            .reserve(BodyId::from(body_id), position)
            .map_err(RuntimeError::from)?;

        reservation.commit();

        self.physics.attachment = Some(PhysicsAttachment::reserved(
            body_id,
            owner_id,
            transform_id,
            BodyProperties::default(),
        ));
        Ok(body_id)
    }

    /// Publish the object catalog entry only after backend publication succeeds.
    pub(super) fn commit_physics_body(&mut self, body_id: ObjectId) -> Result<(), RuntimeError> {
        let attachment = self
            .physics
            .attachment
            .as_mut()
            .ok_or(RuntimeError::InvalidObject)?;
        if attachment.body_id != body_id {
            return Err(RuntimeError::InvalidObject);
        }
        if attachment.committed {
            return Err(RuntimeError::Lifecycle);
        }

        let publication = self.objects.prepare_rigidbody_publication(body_id)?;

        self.physics
            .world
            .commit(BodyId::from(body_id))
            .map_err(RuntimeError::from)?;
        publication.commit();
        attachment.committed = true;
        Ok(())
    }

    /// Abort an unpublished reservation, retaining it if backend abort fails.
    pub(super) fn abort_physics_body(&mut self, body_id: ObjectId) -> Result<(), RuntimeError> {
        let attachment = self.reserved_attachment(body_id)?;
        let cleanup = match self.objects.prepare_retirement(&[body_id]) {
            Ok(cleanup) => cleanup,
            Err(error) => {
                return Err(RuntimeError::from(error));
            }
        };
        let abort_result = self.physics.world.abort(BodyId::from(body_id));
        if let Err(error) = abort_result {
            drop(cleanup);
            return Err(RuntimeError::from(error));
        }

        cleanup.commit();
        self.physics.attachment = None;
        debug_assert_eq!(attachment.body_id, body_id);
        Ok(())
    }

    /// Read a committed body as an owned state copy.
    pub(super) fn read_physics_body(&self, body_id: ObjectId) -> Result<BodyState, RuntimeError> {
        let attachment = self.ensure_attachment_matches(body_id)?;
        if !attachment.committed {
            return Err(RuntimeError::Lifecycle);
        }
        self.physics
            .world
            .body(BodyId::from(body_id))
            .map_err(RuntimeError::from)
    }

    pub(super) fn physics_attachment(&self) -> Option<&PhysicsAttachment> {
        self.physics.attachment.as_ref()
    }

    pub(crate) fn physics_attachment_ids(&self) -> Option<(ObjectId, ObjectId)> {
        self.physics
            .attachment
            .map(|attachment| (attachment.owner_id, attachment.body_id))
    }

    /// Preflight a user-authored position write before mutating TransformData.
    pub(crate) fn prepare_authored_position_write(
        &self,
        transform_id: ObjectId,
    ) -> Result<Option<u64>, RuntimeError> {
        let Some(attachment) = self.physics.attachment else {
            return Ok(None);
        };
        if attachment.transform_id != transform_id {
            return Ok(None);
        }
        attachment
            .properties
            .next_authored_revision()
            .map(Some)
            .map_err(RuntimeError::from)
    }

    /// Commit the revision prepared before the corresponding Transform write.
    pub(crate) fn commit_authored_position_write(&mut self, transform_id: ObjectId, revision: u64) {
        let attachment = self
            .physics
            .attachment
            .as_mut()
            .expect("prepared authored write requires an attachment");
        debug_assert_eq!(attachment.transform_id, transform_id);
        attachment.properties.commit_authored_revision(revision);
    }

    fn ensure_attachment_matches(
        &self,
        body_id: ObjectId,
    ) -> Result<PhysicsAttachment, RuntimeError> {
        let attachment = self.physics.attachment.ok_or(RuntimeError::InvalidObject)?;
        if attachment.body_id != body_id {
            return Err(RuntimeError::InvalidObject);
        }
        Ok(attachment)
    }

    fn reserved_attachment(&self, body_id: ObjectId) -> Result<PhysicsAttachment, RuntimeError> {
        let attachment = self.ensure_attachment_matches(body_id)?;
        if attachment.committed {
            return Err(RuntimeError::Lifecycle);
        }
        Ok(attachment)
    }
}
