//! Reserve, publish, abort, and resolve Runtime physics attachments.
//!
//! Object identity and publication belong to `engine_object`; body state and
//! reservation revisions belong to `engine_physics_3d`. This module composes
//! those owners while keeping every lookup keyed by the exact body ID.
//! Multi-body admission follows the accepted OpenUnity policy
//! `INFRA-PHYSICS-MOTION-MULTI-001`; it does not extend the Unity observation
//! recorded by `ENGINE-PHYSICS-MOTION-001`.

use engine_identifiers::ObjectId;
use engine_math::{Quaternion, Vector};
use engine_physics_api::{BodyId, BodyState};

use crate::{Runtime, RuntimeError};

pub(super) use super::attachment_registry::PhysicsAttachment;
use super::body_properties::BodyProperties;

impl Runtime {
    /// Reserve a hidden Rigidbody and matching backend body.
    ///
    /// Owner uniqueness and Runtime/world capacity are preflighted before
    /// object identity is allocated. The reservation guard removes the hidden
    /// object identity if backend validation fails. The attachment is inserted
    /// only after both object and backend owners hold the reservation.
    pub(super) fn reserve_physics_body(
        &mut self,
        owner_id: ObjectId,
        transform_id: ObjectId,
        position: Vector,
        rotation: Quaternion,
    ) -> Result<ObjectId, RuntimeError> {
        let slot = self.physics.prepare_attachment_slot(owner_id)?;
        self.physics
            .world
            .ensure_reservation_capacity()
            .map_err(RuntimeError::from)?;

        let reservation = self.objects.prepare_rigidbody_reservation(owner_id)?;
        let body_id = reservation.body_id();

        self.physics.ensure_body_id_available(body_id)?;
        self.physics
            .world
            .reserve_pose(BodyId::from(body_id), position, rotation)
            .map_err(RuntimeError::from)?;

        reservation.commit();
        self.physics.insert_reserved_attachment(
            slot,
            PhysicsAttachment::reserved(body_id, owner_id, transform_id, BodyProperties::default()),
        );
        Ok(body_id)
    }

    /// Publish the owner catalog entry after backend publication succeeds.
    pub(super) fn commit_physics_body(&mut self, body_id: ObjectId) -> Result<(), RuntimeError> {
        let attachment = self.reserved_attachment(body_id)?;
        let index = self
            .physics
            .attachment_index_by_body(body_id)
            .ok_or(RuntimeError::InvalidObject)?;
        let publication = self.objects.prepare_rigidbody_publication(body_id)?;

        self.physics
            .world
            .commit(BodyId::from(body_id))
            .map_err(RuntimeError::from)?;

        publication.commit();
        self.physics.commit_attachment(index, body_id);
        debug_assert_eq!(attachment.body_id, body_id);
        Ok(())
    }

    /// Abort an unpublished reservation, retaining it if backend abort fails.
    pub(super) fn abort_physics_body(&mut self, body_id: ObjectId) -> Result<(), RuntimeError> {
        self.reserved_attachment(body_id)?;
        let index = self
            .physics
            .attachment_index_by_body(body_id)
            .ok_or(RuntimeError::InvalidObject)?;
        let cleanup = self.objects.prepare_retirement(&[body_id])?;

        if let Err(error) = self.physics.world.abort(BodyId::from(body_id)) {
            drop(cleanup);
            return Err(RuntimeError::from(error));
        }

        cleanup.commit();
        self.physics.remove_attachment(index, body_id);
        Ok(())
    }

    /// Read committed backend state after resolving this exact body ID.
    pub(super) fn read_physics_body(&self, body_id: ObjectId) -> Result<BodyState, RuntimeError> {
        let attachment = self.physics_attachment(body_id)?;
        if !attachment.committed {
            return Err(RuntimeError::Lifecycle);
        }

        self.physics
            .world
            .body(BodyId::from(body_id))
            .map_err(RuntimeError::from)
    }

    pub(super) fn physics_attachment(
        &self,
        body_id: ObjectId,
    ) -> Result<&PhysicsAttachment, RuntimeError> {
        let attachment = self
            .physics
            .attachment_by_body(body_id)
            .ok_or(RuntimeError::InvalidObject)?;

        if self.objects.kind(body_id)? != crate::ObjectKind::Rigidbody
            || self.objects.component_gameobject(body_id)? != attachment.owner_id
            || self.objects.gameobject_transform(attachment.owner_id)? != attachment.transform_id
        {
            return Err(RuntimeError::InvalidObject);
        }

        Ok(attachment)
    }

    pub(crate) fn physics_attachment_ids(&self) -> impl Iterator<Item = (ObjectId, ObjectId)> + '_ {
        self.physics
            .attachments()
            .map(|attachment| (attachment.owner_id, attachment.body_id))
    }

    pub(crate) fn physics_has_attachment_for_owner(&self, owner_id: ObjectId) -> bool {
        self.physics.attachment_by_owner(owner_id).is_some()
    }

    /// Preflight a user-authored position write before mutating TransformData.
    pub(crate) fn prepare_authored_position_write(
        &self,
        transform_id: ObjectId,
    ) -> Result<Option<u64>, RuntimeError> {
        let Some(attachment) = self.physics.attachment_by_transform(transform_id) else {
            return Ok(None);
        };

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
            .attachment_by_transform_mut(transform_id)
            .expect("prepared authored write retains its exact Transform attachment");
        attachment.properties.commit_authored_revision(revision);
    }

    fn reserved_attachment(&self, body_id: ObjectId) -> Result<PhysicsAttachment, RuntimeError> {
        let attachment = *self.physics_attachment(body_id)?;
        if attachment.committed {
            return Err(RuntimeError::Lifecycle);
        }
        Ok(attachment)
    }
}
