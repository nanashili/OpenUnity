//! Transactional collision stepping and complete Transform publication.

use engine_identifiers::ObjectId;
use engine_math::{Quaternion, Vector, finite, normalize};

use super::PhysicsSceneBodyState;
use super::scene_state::{CollisionBackend, CollisionSceneState};
use crate::{ObjectKind, Runtime, RuntimeError};

#[derive(Clone, Copy)]
struct TransformPublication {
    transform: ObjectId,
    body: PhysicsSceneBodyState,
}

impl Runtime {
    /// Advance the selected collision scene and publish every copied body pose.
    ///
    /// The runtime stages the solver candidate and validates all destinations
    /// before replacing any live solver or Transform state.
    pub fn physics_scene_step(&mut self, timestep: f32) -> Result<(), RuntimeError> {
        self.lifecycle_ready()?;
        if self.scene_staging() {
            return Err(RuntimeError::StagingBusy);
        }

        let mut candidate = self.collision_scene.clone();
        candidate.step(timestep)?;
        let publications = self.prepare_transform_publications(&candidate)?;

        self.collision_scene = candidate;
        for publication in publications {
            let transform = self
                .transforms
                .get_mut(&publication.transform)
                .expect("collision Transform destinations were preflighted");
            transform.position = publication.body.position();
            transform.rotation = publication.body.rotation();
        }

        Ok(())
    }

    /// Copy one admitted body's last solver state without exposing Rapier types.
    pub fn physics_scene_body_state(
        &self,
        owner: ObjectId,
    ) -> Result<PhysicsSceneBodyState, RuntimeError> {
        self.collision_scene_admission_ready()?;
        self.validate_collision_owner(owner)?;
        let body = self.collision_scene.body_state(owner)?;
        validate_body_state(body)?;
        Ok(body)
    }

    fn prepare_transform_publications(
        &self,
        scene: &CollisionSceneState,
    ) -> Result<Vec<TransformPublication>, RuntimeError> {
        let mut publications = Vec::new();
        publications
            .try_reserve_exact(scene.body_count())
            .map_err(|_| RuntimeError::CountOverflow)?;

        let bodies = match scene.backend.as_ref().ok_or(RuntimeError::Lifecycle)? {
            CollisionBackend::TwoD { world, bodies } => {
                if world.body_count() != bodies.len() {
                    return Err(RuntimeError::Lifecycle);
                }
                bodies
            }
            CollisionBackend::ThreeD { world, bodies } => {
                if world.body_count() != bodies.len() {
                    return Err(RuntimeError::Lifecycle);
                }
                bodies
            }
        };

        for (raw_owner, binding) in bodies {
            let owner = ObjectId::from_raw(*raw_owner).ok_or(RuntimeError::InvalidObject)?;
            self.validate_collision_owner(owner)?;
            let transform = self.transform(binding.transform)?;
            if transform.owner != owner || transform.parent.is_some() {
                return Err(RuntimeError::Lifecycle);
            }

            let body = scene.body_state(owner)?;
            validate_body_state(body)?;
            publications.push(TransformPublication {
                transform: binding.transform,
                body,
            });
        }

        Ok(publications)
    }

    fn validate_collision_owner(&self, owner: ObjectId) -> Result<(), RuntimeError> {
        if self.objects.kind(owner)? != ObjectKind::GameObject {
            return Err(RuntimeError::InvalidObject);
        }
        if self.lifecycle_pending_owner(owner) {
            return Err(RuntimeError::Lifecycle);
        }

        let binding = self.collision_scene.binding(owner)?;
        let transform = self.transform(binding.transform)?;
        if transform.owner != owner
            || transform.parent.is_some()
            || self.gameobject_transform(owner)? != binding.transform
        {
            return Err(RuntimeError::Lifecycle);
        }

        Ok(())
    }
}

impl CollisionSceneState {
    fn step(&mut self, timestep: f32) -> Result<(), RuntimeError> {
        match self.backend.as_mut().ok_or(RuntimeError::Lifecycle)? {
            CollisionBackend::TwoD { world, .. } => world.step(timestep)?,
            CollisionBackend::ThreeD { world, .. } => world.step(timestep)?,
        }
        Ok(())
    }
}

fn validate_body_state(body: PhysicsSceneBodyState) -> Result<(), RuntimeError> {
    finite_vector(body.position())?;
    finite_vector(body.linear_velocity())?;
    finite_quaternion(body.rotation())?;
    normalize(body.rotation())?;
    Ok(())
}

fn finite_vector(value: Vector) -> Result<(), RuntimeError> {
    finite(&value)?;
    Ok(())
}

fn finite_quaternion(value: Quaternion) -> Result<(), RuntimeError> {
    if value.iter().all(|component| component.is_finite()) {
        Ok(())
    } else {
        Err(RuntimeError::Numeric)
    }
}
