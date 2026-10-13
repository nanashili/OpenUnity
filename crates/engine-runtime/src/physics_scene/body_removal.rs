//! Explicit collision-body removal by its owning runtime GameObject.

use engine_identifiers::ObjectId;
use engine_physics_api::BodyId;

use super::scene_state::{CollisionBackend, CollisionSceneState};
use crate::{Runtime, RuntimeError};

impl Runtime {
    /// Remove a body by its owning GameObject identity.
    pub fn physics_scene_remove_body(&mut self, owner: ObjectId) -> Result<(), RuntimeError> {
        self.collision_scene_admission_ready()?;

        let mut candidate = self.collision_scene.clone();
        candidate.remove_body(owner)?;
        self.collision_scene = candidate;
        Ok(())
    }
}

impl CollisionSceneState {
    pub(super) fn remove_body(&mut self, owner: ObjectId) -> Result<(), RuntimeError> {
        let body_id = BodyId::from(owner);
        match self.backend.as_mut().ok_or(RuntimeError::Lifecycle)? {
            CollisionBackend::TwoD { world, bodies } => {
                if !bodies.contains_key(&owner.to_raw()) {
                    return Err(RuntimeError::InvalidObject);
                }
                world.remove(body_id)?;
                bodies.remove(&owner.to_raw());
            }
            CollisionBackend::ThreeD { world, bodies } => {
                if !bodies.contains_key(&owner.to_raw()) {
                    return Err(RuntimeError::InvalidObject);
                }
                world.remove(body_id)?;
                bodies.remove(&owner.to_raw());
            }
        }

        Ok(())
    }
}
