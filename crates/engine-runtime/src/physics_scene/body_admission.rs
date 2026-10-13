//! Bind runtime GameObject Transforms to dimension-specific solver bodies.

use engine_identifiers::ObjectId;
use engine_math::{Quaternion, Vector, normalize};
use engine_physics_api::BodyId;

use super::scene_state::{CollisionBackend, CollisionBodyBinding, CollisionSceneState};
use crate::{ObjectKind, Runtime, RuntimeError};

impl Runtime {
    /// Admit a dynamic 2D box using a GameObject's root Transform as its pose.
    pub fn physics_scene_add_dynamic_box_2d(
        &mut self,
        owner: ObjectId,
        half_extents: [f32; 2],
    ) -> Result<(), RuntimeError> {
        self.add_collision_box_2d(owner, half_extents, true)
    }

    /// Admit a fixed 2D box using a GameObject's root Transform as its pose.
    pub fn physics_scene_add_fixed_box_2d(
        &mut self,
        owner: ObjectId,
        half_extents: [f32; 2],
    ) -> Result<(), RuntimeError> {
        self.add_collision_box_2d(owner, half_extents, false)
    }

    /// Admit a dynamic 3D cuboid using a GameObject's root Transform as its pose.
    pub fn physics_scene_add_dynamic_cuboid_3d(
        &mut self,
        owner: ObjectId,
        half_extents: Vector,
    ) -> Result<(), RuntimeError> {
        self.add_collision_cuboid_3d(owner, half_extents, true)
    }

    /// Admit a fixed 3D cuboid using a GameObject's root Transform as its pose.
    pub fn physics_scene_add_fixed_cuboid_3d(
        &mut self,
        owner: ObjectId,
        half_extents: Vector,
    ) -> Result<(), RuntimeError> {
        self.add_collision_cuboid_3d(owner, half_extents, false)
    }

    fn add_collision_box_2d(
        &mut self,
        owner: ObjectId,
        half_extents: [f32; 2],
        dynamic: bool,
    ) -> Result<(), RuntimeError> {
        self.collision_scene_admission_ready()?;
        let transform_id = self.collision_owner_transform(owner)?;
        let transform = self.transform(transform_id)?;
        let position = transform.position;
        let rotation = normalize(transform.rotation)?;

        let mut candidate = self.collision_scene.clone();
        candidate.insert_box_2d(
            owner,
            transform_id,
            position,
            rotation,
            half_extents,
            dynamic,
        )?;
        self.collision_scene = candidate;
        Ok(())
    }

    fn add_collision_cuboid_3d(
        &mut self,
        owner: ObjectId,
        half_extents: Vector,
        dynamic: bool,
    ) -> Result<(), RuntimeError> {
        self.collision_scene_admission_ready()?;
        let transform_id = self.collision_owner_transform(owner)?;
        let transform = self.transform(transform_id)?;
        let position = transform.position;
        let rotation = normalize(transform.rotation)?;

        let mut candidate = self.collision_scene.clone();
        candidate.insert_cuboid_3d(
            owner,
            transform_id,
            position,
            rotation,
            half_extents,
            dynamic,
        )?;
        self.collision_scene = candidate;
        Ok(())
    }

    fn collision_owner_transform(&self, owner: ObjectId) -> Result<ObjectId, RuntimeError> {
        if self.objects.kind(owner)? != ObjectKind::GameObject {
            return Err(RuntimeError::InvalidKind);
        }
        if self.physics.has_attachment_for_owner(owner) {
            return Err(RuntimeError::PhysicsUnsupported);
        }
        if self.lifecycle_pending_owner(owner) {
            return Err(RuntimeError::Lifecycle);
        }
        if !self.gameobject_active(owner, true)? {
            return Err(RuntimeError::Lifecycle);
        }

        let transform_id = self.gameobject_transform(owner)?;
        self.validate_root_structure(transform_id)?;
        let transform = self.transform(transform_id)?;
        if transform.owner != owner {
            return Err(RuntimeError::InvalidObject);
        }

        Ok(transform_id)
    }
}

impl CollisionSceneState {
    fn insert_box_2d(
        &mut self,
        owner: ObjectId,
        transform: ObjectId,
        position: Vector,
        rotation: Quaternion,
        half_extents: [f32; 2],
        dynamic: bool,
    ) -> Result<(), RuntimeError> {
        let body_id = BodyId::from(owner);
        let binding = CollisionBodyBinding {
            transform,
            original_z: position[2],
            dynamic,
        };

        match self.backend.as_mut().ok_or(RuntimeError::Lifecycle)? {
            CollisionBackend::TwoD { world, bodies } => {
                let planar_angle = planar_angle(rotation)?;
                let planar_position = [position[0], position[1]];
                if dynamic {
                    world.insert_dynamic_box(
                        body_id,
                        planar_position,
                        planar_angle,
                        half_extents,
                    )?;
                } else {
                    world.insert_fixed_box(body_id, planar_position, planar_angle, half_extents)?;
                }
                bodies.insert(owner.to_raw(), binding);
            }
            CollisionBackend::ThreeD { .. } => return Err(RuntimeError::PhysicsUnsupported),
        }

        Ok(())
    }

    fn insert_cuboid_3d(
        &mut self,
        owner: ObjectId,
        transform: ObjectId,
        position: Vector,
        rotation: Quaternion,
        half_extents: Vector,
        dynamic: bool,
    ) -> Result<(), RuntimeError> {
        let body_id = BodyId::from(owner);
        let binding = CollisionBodyBinding {
            transform,
            original_z: position[2],
            dynamic,
        };

        match self.backend.as_mut().ok_or(RuntimeError::Lifecycle)? {
            CollisionBackend::TwoD { .. } => return Err(RuntimeError::PhysicsUnsupported),
            CollisionBackend::ThreeD { world, bodies } => {
                if dynamic {
                    world.insert_dynamic_cuboid(body_id, position, rotation, half_extents)?;
                } else {
                    world.insert_fixed_cuboid(body_id, position, rotation, half_extents)?;
                }
                bodies.insert(owner.to_raw(), binding);
            }
        }

        Ok(())
    }
}

fn planar_angle(rotation: Quaternion) -> Result<f32, RuntimeError> {
    if rotation[0] != 0.0 || rotation[1] != 0.0 {
        return Err(RuntimeError::PhysicsUnsupported);
    }

    Ok(2.0 * rotation[2].atan2(rotation[3]))
}
