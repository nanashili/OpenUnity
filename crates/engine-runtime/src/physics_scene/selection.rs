//! Selection and configuration of one runtime-owned collision dimension.

use engine_math::Vector;
use engine_physics_2d::CollisionWorld2D;
use engine_physics_3d::CollisionWorld3D;

use super::PhysicsSceneDimension;
use super::scene_state::{CollisionBackend, CollisionSceneState};
use crate::{Runtime, RuntimeError};

impl Runtime {
    /// Select an empty 2D collision scene with explicit gravity.
    pub fn physics_scene_create_2d(&mut self, gravity: [f32; 2]) -> Result<(), RuntimeError> {
        self.collision_scene_admission_ready()?;
        self.collision_scene.create_2d(gravity)
    }

    /// Select an empty 3D collision scene with explicit gravity.
    pub fn physics_scene_create_3d(&mut self, gravity: Vector) -> Result<(), RuntimeError> {
        self.collision_scene_admission_ready()?;
        self.collision_scene.create_3d(gravity)
    }

    /// Read the selected collision profile, if one has been created.
    pub fn physics_scene_dimension(&self) -> Option<PhysicsSceneDimension> {
        self.collision_scene.dimension()
    }

    /// Return the number of bodies admitted to the selected scene.
    pub fn physics_scene_body_count(&self) -> usize {
        self.collision_scene.body_count()
    }

    /// Change gravity without replacing the selected world or its bodies.
    pub fn physics_scene_set_gravity_2d(&mut self, gravity: [f32; 2]) -> Result<(), RuntimeError> {
        self.collision_scene_admission_ready()?;
        self.collision_scene.set_gravity_2d(gravity)
    }

    /// Change 3D gravity without replacing the selected world or its bodies.
    pub fn physics_scene_set_gravity_3d(&mut self, gravity: Vector) -> Result<(), RuntimeError> {
        self.collision_scene_admission_ready()?;
        self.collision_scene.set_gravity_3d(gravity)
    }
}

impl CollisionSceneState {
    fn create_2d(&mut self, gravity: [f32; 2]) -> Result<(), RuntimeError> {
        self.ensure_unselected()?;

        let world = CollisionWorld2D::new(gravity)?;
        self.backend = Some(CollisionBackend::TwoD {
            world: Box::new(world),
            bodies: Default::default(),
        });
        Ok(())
    }

    fn create_3d(&mut self, gravity: Vector) -> Result<(), RuntimeError> {
        self.ensure_unselected()?;

        let world = CollisionWorld3D::new(gravity)?;
        self.backend = Some(CollisionBackend::ThreeD {
            world: Box::new(world),
            bodies: Default::default(),
        });
        Ok(())
    }

    fn ensure_unselected(&self) -> Result<(), RuntimeError> {
        if self.backend.is_some() {
            return Err(RuntimeError::Lifecycle);
        }
        Ok(())
    }

    fn set_gravity_2d(&mut self, gravity: [f32; 2]) -> Result<(), RuntimeError> {
        match self.backend.as_mut().ok_or(RuntimeError::Lifecycle)? {
            CollisionBackend::TwoD { world, .. } => world.set_gravity(gravity)?,
            CollisionBackend::ThreeD { .. } => return Err(RuntimeError::PhysicsUnsupported),
        }
        Ok(())
    }

    fn set_gravity_3d(&mut self, gravity: Vector) -> Result<(), RuntimeError> {
        match self.backend.as_mut().ok_or(RuntimeError::Lifecycle)? {
            CollisionBackend::TwoD { .. } => return Err(RuntimeError::PhysicsUnsupported),
            CollisionBackend::ThreeD { world, .. } => world.set_gravity(gravity)?,
        }
        Ok(())
    }
}
