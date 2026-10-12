//! Motion retirement with borrow-bound ObjectStore cleanup.

use engine_object::{ObjectId, ObjectStore, PreparedObjectRelease, PreparedObjectRetirement};
use engine_physics_3d::MotionWorld;
use engine_physics_api::{BodyId, PhysicsError};

use super::PhysicsState;
use super::attachment::PhysicsAttachment;

use crate::{Runtime, RuntimeError};

/// Whether an attachment is retired by its own component ID or its owner ID.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AttachmentTarget {
    Body,
    Owner,
}

/// Pure retirement planning result. It performs no object or physics mutation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RetirementPlan {
    pub body: BodyId,
    pub owner: ObjectId,
    pub target: Option<AttachmentTarget>,
}

/// Errors preserve whether object preparation or backend retirement failed.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RetirementError {
    Object(engine_object::ObjectError),
    Physics(PhysicsError),
}

impl From<engine_object::ObjectError> for RetirementError {
    fn from(error: engine_object::ObjectError) -> Self {
        Self::Object(error)
    }
}

impl From<PhysicsError> for RetirementError {
    fn from(error: PhysicsError) -> Self {
        Self::Physics(error)
    }
}

/// Copy the target decision before either store is mutably borrowed.
pub fn plan_attachment(body: BodyId, owner: ObjectId, ids: &[ObjectId]) -> RetirementPlan {
    let body_id = ObjectId::from(body);
    let target = if ids.contains(&body_id) {
        Some(AttachmentTarget::Body)
    } else if ids.contains(&owner) {
        Some(AttachmentTarget::Owner)
    } else {
        None
    };
    RetirementPlan {
        body,
        owner,
        target,
    }
}

/// Prepare ordinary owner/component retirement and commit all stores once.
pub fn prepare_retirement(
    objects: &mut ObjectStore,
    physics: &mut PhysicsState,
    ids: &[ObjectId],
) -> Result<(), RetirementError> {
    let plan = current_plan(physics, ids);
    let guard = objects.prepare_retirement(ids)?;
    finish_retirement(guard, physics, plan)
}

/// Prepare complete scene release and commit all stores once.
pub fn prepare_release(
    objects: &mut ObjectStore,
    physics: &mut PhysicsState,
    ids: &[ObjectId],
) -> Result<(), RetirementError> {
    let plan = current_plan(physics, ids);
    let guard = objects.prepare_release(ids)?;
    finish_release(guard, physics, plan)
}

impl Runtime {
    pub(crate) fn physics_retire_objects(&mut self, ids: &[ObjectId]) -> Result<(), RuntimeError> {
        prepare_retirement(&mut self.objects, &mut self.physics, ids).map_err(map_retirement_error)
    }

    pub(crate) fn physics_release_objects(&mut self, ids: &[ObjectId]) -> Result<(), RuntimeError> {
        prepare_release(&mut self.objects, &mut self.physics, ids).map_err(map_retirement_error)
    }
}

fn map_retirement_error(error: RetirementError) -> RuntimeError {
    match error {
        RetirementError::Object(error) => error.into(),
        RetirementError::Physics(error) => error.into(),
    }
}

fn current_plan(physics: &PhysicsState, ids: &[ObjectId]) -> Option<RetirementPlan> {
    physics.attachment.map(|attachment| {
        plan_attachment(BodyId::from(attachment.body_id), attachment.owner_id, ids)
    })
}

trait ObjectGuard {
    fn commit_guard(self);
}

impl ObjectGuard for PreparedObjectRetirement<'_> {
    fn commit_guard(self) {
        self.commit();
    }
}

impl ObjectGuard for PreparedObjectRelease<'_> {
    fn commit_guard(self) {
        self.commit();
    }
}

fn finish_retirement<G: ObjectGuard>(
    guard: G,
    physics: &mut PhysicsState,
    plan: Option<RetirementPlan>,
) -> Result<(), RetirementError> {
    retire_backend(&mut physics.world, physics.attachment.as_ref(), plan)?;
    guard.commit_guard();
    clear_targeted_attachment(physics, plan);
    Ok(())
}

fn finish_release(
    guard: PreparedObjectRelease<'_>,
    physics: &mut PhysicsState,
    plan: Option<RetirementPlan>,
) -> Result<(), RetirementError> {
    retire_backend(&mut physics.world, physics.attachment.as_ref(), plan)?;
    guard.commit();
    clear_targeted_attachment(physics, plan);
    Ok(())
}

fn retire_backend(
    world: &mut MotionWorld,
    attachment: Option<&PhysicsAttachment>,
    plan: Option<RetirementPlan>,
) -> Result<(), RetirementError> {
    let Some(plan) = plan else {
        return Ok(());
    };
    if plan.target.is_none() {
        return Ok(());
    }
    let current = attachment.ok_or(PhysicsError::InvalidBody)?;
    let body = BodyId::from(current.body_id);
    if body != plan.body {
        return Err(PhysicsError::InvalidBody.into());
    }
    if current.committed {
        world.remove(body)?;
    } else {
        world.abort(body)?;
    }
    Ok(())
}

fn clear_targeted_attachment(physics: &mut PhysicsState, plan: Option<RetirementPlan>) {
    if plan.is_some_and(|planned| planned.target.is_some()) {
        physics.attachment = None;
    }
}
