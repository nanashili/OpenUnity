//! Select exact attachment identities before Runtime retirement begins.

use engine_identifiers::ObjectId;
use engine_object::{ObjectError, ObjectKind, ObjectStore};
use engine_physics_api::{BodyId, PhysicsError};

use super::attachment::PhysicsAttachment;
use super::attachment_registry::{AttachmentIndex, PhysicsState};

use super::retirement::RetirementError;

/// Exact body and registry identities selected for one owner/component request.
pub(super) struct RetirementPlan {
    pub(super) bodies: Vec<BodyId>,
    pub(super) attachment_indices: Vec<(AttachmentIndex, ObjectId)>,
}

/// Copy requested owner identities for the query-world preparation.
pub(super) fn query_owners(
    objects: &ObjectStore,
    ids: &[ObjectId],
) -> Result<Vec<ObjectId>, RetirementError> {
    let mut owners = Vec::new();
    owners
        .try_reserve_exact(ids.len())
        .map_err(|_| RetirementError::Object(ObjectError::CapacityExhausted))?;

    for &id in ids {
        match objects.kind(id) {
            Ok(ObjectKind::GameObject) => owners.push(id),
            Ok(_) | Err(ObjectError::InvalidObject) => {}
            Err(error) => return Err(RetirementError::Object(error)),
        }
    }

    Ok(owners)
}

/// Validate exact ObjectStore relationships and collect attachments by body ID.
pub(super) fn plan_retirement(
    objects: &ObjectStore,
    physics: &PhysicsState,
    ids: &[ObjectId],
) -> Result<RetirementPlan, RetirementError> {
    let mut plan = RetirementPlan::try_new()?;

    for attachment in physics.ordered_attachments() {
        if !is_targeted(attachment, ids) {
            continue;
        }

        validate_object_attachment(objects, attachment)?;
        validate_world_attachment(physics, attachment)?;

        let index = physics
            .attachment_index_by_body(attachment.body_id)
            .ok_or(PhysicsError::InvalidBody)?;
        plan.bodies.push(BodyId::from(attachment.body_id));
        plan.attachment_indices.push((index, attachment.body_id));
    }

    Ok(plan)
}

fn is_targeted(attachment: PhysicsAttachment, ids: &[ObjectId]) -> bool {
    ids.contains(&attachment.body_id) || ids.contains(&attachment.owner_id)
}

fn validate_object_attachment(
    objects: &ObjectStore,
    attachment: PhysicsAttachment,
) -> Result<(), RetirementError> {
    if objects.kind(attachment.body_id)? != ObjectKind::Rigidbody
        || objects.component_gameobject(attachment.body_id)? != attachment.owner_id
        || objects.gameobject_transform(attachment.owner_id)? != attachment.transform_id
        || objects.is_component_published(attachment.body_id)? != attachment.committed
    {
        return Err(ObjectError::ComponentState.into());
    }

    Ok(())
}

fn validate_world_attachment(
    physics: &PhysicsState,
    attachment: PhysicsAttachment,
) -> Result<(), RetirementError> {
    if attachment.committed {
        physics
            .world
            .body(BodyId::from(attachment.body_id))
            .map_err(RetirementError::Physics)?;
    } else {
        physics
            .world
            .validate_reservation(BodyId::from(attachment.body_id))
            .map_err(RetirementError::Physics)?;
    }

    Ok(())
}

impl RetirementPlan {
    fn try_new() -> Result<Self, RetirementError> {
        let mut bodies = Vec::new();
        bodies
            .try_reserve_exact(super::attachment_registry::MAX_PHYSICS_ATTACHMENTS)
            .map_err(|_| RetirementError::Object(ObjectError::CapacityExhausted))?;

        let mut attachment_indices = Vec::new();
        attachment_indices
            .try_reserve_exact(super::attachment_registry::MAX_PHYSICS_ATTACHMENTS)
            .map_err(|_| RetirementError::Object(ObjectError::CapacityExhausted))?;

        Ok(Self {
            bodies,
            attachment_indices,
        })
    }
}
