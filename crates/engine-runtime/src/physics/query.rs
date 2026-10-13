//! Runtime ownership and admission for explicit world-space AABB queries.

use engine_identifiers::ObjectId;
use engine_math::{Aabb, MathError, Vector};
use engine_physics_api::RaySegmentHit;

use crate::{ObjectKind, Runtime, RuntimeError};

impl Runtime {
    /// Register one explicit query box for a live GameObject owner.
    pub fn physics_aabb_add(
        &mut self,
        owner: ObjectId,
        min: Vector,
        max: Vector,
    ) -> Result<(), RuntimeError> {
        self.reject_query_during_staging()?;
        self.validate_query_owner(owner)?;

        self.physics
            .query
            .add_aabb_from_vectors(owner, min, max)
            .map_err(RuntimeError::from)
    }

    /// Return the nearest hit on a closed segment, if one exists.
    pub fn physics_raycast_segment(
        &self,
        origin: Vector,
        displacement: Vector,
    ) -> Result<Option<RaySegmentHit>, RuntimeError> {
        self.reject_query_during_staging()?;

        self.physics
            .query
            .raycast_segment(origin, displacement)
            .map_err(RuntimeError::from)
    }

    /// Return all registered owners whose boxes overlap the query box.
    pub fn physics_overlap_aabb(
        &self,
        min: Vector,
        max: Vector,
    ) -> Result<Vec<ObjectId>, RuntimeError> {
        self.reject_query_during_staging()?;
        let bounds = Aabb::try_new(min, max).map_err(map_query_bounds_error)?;

        self.physics
            .query
            .overlap_aabb(&bounds)
            .map_err(RuntimeError::from)
    }

    fn reject_query_during_staging(&self) -> Result<(), RuntimeError> {
        if self.scene_staging() {
            return Err(RuntimeError::StagingBusy);
        }
        Ok(())
    }

    fn validate_query_owner(&self, owner: ObjectId) -> Result<(), RuntimeError> {
        if self.objects.kind(owner)? != ObjectKind::GameObject {
            return Err(RuntimeError::InvalidKind);
        }
        if self.owner_pending_destroy(owner)? {
            return Err(RuntimeError::Lifecycle);
        }
        Ok(())
    }
}

fn map_query_bounds_error(error: MathError) -> RuntimeError {
    match error {
        MathError::Numeric => RuntimeError::Numeric,
        MathError::Unsupported => RuntimeError::InvalidPhysicsQuery,
    }
}
