//! Prepare and commit cross-store body retirement batches.

use engine_identifiers::ObjectId;
use engine_object::{ObjectStore, PreparedObjectRelease, PreparedObjectRetirement};
use engine_physics_3d::{MotionWorld, PreparedOwnerRetirement};
use engine_physics_api::{PhysicsError, QueryError};

use super::attachment_registry::PhysicsState;
use super::retirement_plan::{RetirementPlan, plan_retirement, query_owners};

use crate::{Runtime, RuntimeError};

/// Errors preserve which owner rejected a retirement batch.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum RetirementError {
    Object(engine_object::ObjectError),
    Physics(PhysicsError),
    Query(QueryError),
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

impl From<QueryError> for RetirementError {
    fn from(error: QueryError) -> Self {
        Self::Query(error)
    }
}

/// Prepare ordinary object retirement and atomically retire matching bodies.
pub(super) fn prepare_retirement(
    objects: &mut ObjectStore,
    physics: &mut PhysicsState,
    ids: &[ObjectId],
) -> Result<(), RetirementError> {
    let plan = plan_retirement(objects, physics, ids)?;
    let owners = query_owners(objects, ids)?;
    let query_guard = physics.query.prepare_owner_retirement(&owners)?;
    let object_guard = objects.prepare_retirement(ids)?;

    commit_transaction(&mut physics.world, object_guard, query_guard, &plan)?;
    clear_retired_attachments(physics, plan);
    Ok(())
}

/// Prepare complete scene release and atomically retire its matching bodies.
pub(super) fn prepare_release(
    objects: &mut ObjectStore,
    physics: &mut PhysicsState,
    ids: &[ObjectId],
) -> Result<(), RetirementError> {
    let plan = plan_retirement(objects, physics, ids)?;
    let owners = query_owners(objects, ids)?;
    let query_guard = physics.query.prepare_owner_retirement(&owners)?;
    let object_guard = objects.prepare_release(ids)?;

    commit_transaction(&mut physics.world, object_guard, query_guard, &plan)?;
    clear_retired_attachments(physics, plan);
    Ok(())
}

impl Runtime {
    pub(crate) fn physics_retire_objects(&mut self, ids: &[ObjectId]) -> Result<(), RuntimeError> {
        let collision_candidate = self.collision_scene.prepare_retirement(ids)?;
        prepare_retirement(&mut self.objects, &mut self.physics, ids)
            .map_err(map_retirement_error)?;
        self.collision_scene = collision_candidate;
        Ok(())
    }

    pub(crate) fn physics_release_objects(&mut self, ids: &[ObjectId]) -> Result<(), RuntimeError> {
        let collision_candidate = self.collision_scene.prepare_retirement(ids)?;
        prepare_release(&mut self.objects, &mut self.physics, ids).map_err(map_retirement_error)?;
        self.collision_scene = collision_candidate;
        Ok(())
    }
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

fn commit_transaction<G: ObjectGuard>(
    world: &mut MotionWorld,
    object_guard: G,
    query_guard: PreparedOwnerRetirement<'_>,
    plan: &RetirementPlan,
) -> Result<(), RetirementError> {
    world.retire_bodies(&plan.bodies)?;

    query_guard.commit();
    object_guard.commit_guard();
    Ok(())
}

fn clear_retired_attachments(physics: &mut PhysicsState, plan: RetirementPlan) {
    for (index, body_id) in plan.attachment_indices {
        physics.remove_attachment(index, body_id);
    }
}

fn map_retirement_error(error: RetirementError) -> RuntimeError {
    match error {
        RetirementError::Object(error) => error.into(),
        RetirementError::Physics(error) => error.into(),
        RetirementError::Query(error) => error.into(),
    }
}
