//! Engine identity mapping and body lifecycle operations.

use engine_physics_api::{BodyId, CollisionError};
use rapier2d::prelude::{ColliderBuilder, RigidBodyBuilder, RigidBodyHandle, Vector};

use super::{CollisionWorld2D, MAX_COLLISION_BODIES_2D};
use crate::body_snapshot::{self, BodySnapshot2D};
use crate::validation::validate_box;

impl CollisionWorld2D {
    /// Add a dynamic box body at the supplied world-space pose.
    ///
    /// Each finite position component must be within
    /// MAX_ABSOLUTE_POSITION_COMPONENT. Each half-extent must be between
    /// MIN_CUBOID_HALF_EXTENT and MAX_CUBOID_HALF_EXTENT. The world admits at
    /// most MAX_COLLISION_BODIES_2D bodies.
    ///
    /// # Errors
    /// Returns InvalidNumber, InvalidDimensions, BodyAlreadyExists, or Capacity.
    pub fn insert_dynamic_box(
        &mut self,
        id: BodyId,
        position: [f32; 2],
        rotation_radians: f32,
        half_extents: [f32; 2],
    ) -> Result<(), CollisionError> {
        self.insert_box(
            id,
            position,
            rotation_radians,
            half_extents,
            RigidBodyBuilder::dynamic(),
        )
    }

    /// Add a fixed box body at the supplied world-space pose.
    ///
    /// Each finite position component must be within
    /// MAX_ABSOLUTE_POSITION_COMPONENT. Each half-extent must be between
    /// MIN_CUBOID_HALF_EXTENT and MAX_CUBOID_HALF_EXTENT. The world admits at
    /// most MAX_COLLISION_BODIES_2D bodies.
    ///
    /// # Errors
    /// Returns InvalidNumber, InvalidDimensions, BodyAlreadyExists, or Capacity.
    pub fn insert_fixed_box(
        &mut self,
        id: BodyId,
        position: [f32; 2],
        rotation_radians: f32,
        half_extents: [f32; 2],
    ) -> Result<(), CollisionError> {
        self.insert_box(
            id,
            position,
            rotation_radians,
            half_extents,
            RigidBodyBuilder::fixed(),
        )
    }

    /// Copy the finite state of one body.
    ///
    /// # Errors
    /// Returns InvalidBody when id is not owned by this world and InvalidNumber
    /// when the solver has produced a nonfinite state.
    pub fn body(&self, id: BodyId) -> Result<BodySnapshot2D, CollisionError> {
        let handle = self.body_handle(id)?;
        let body = self
            .rigid_bodies
            .get(handle)
            .ok_or(CollisionError::InvalidBody)?;

        body_snapshot::copy_from(body)
    }

    /// Remove one body and its attached collider from the world.
    ///
    /// # Errors
    /// Returns InvalidBody when id is not owned by this world.
    pub fn remove(&mut self, id: BodyId) -> Result<(), CollisionError> {
        let handle = self.body_handle(id)?;
        let removed_body = self.rigid_bodies.remove(
            handle,
            &mut self.islands,
            &mut self.colliders,
            &mut self.impulse_joints,
            &mut self.multibody_joints,
            true,
        );

        if removed_body.is_none() {
            return Err(CollisionError::InvalidBody);
        }

        let removed_handle = self
            .body_handles
            .remove(&id)
            .ok_or(CollisionError::InvalidBody)?;
        debug_assert_eq!(removed_handle, handle);

        Ok(())
    }

    /// Return the number of currently owned bodies.
    pub fn body_count(&self) -> usize {
        self.body_handles.len()
    }

    fn insert_box(
        &mut self,
        id: BodyId,
        position: [f32; 2],
        rotation_radians: f32,
        half_extents: [f32; 2],
        body_builder: RigidBodyBuilder,
    ) -> Result<(), CollisionError> {
        validate_box(position, rotation_radians, half_extents)?;
        self.ensure_body_slot(id)?;

        let body = body_builder
            .translation(Vector::new(position[0], position[1]))
            .rotation(rotation_radians)
            .build();
        let handle = self.rigid_bodies.insert(body);
        let collider = ColliderBuilder::cuboid(half_extents[0], half_extents[1]).build();
        self.colliders
            .insert_with_parent(collider, handle, &mut self.rigid_bodies);

        self.body_handles.insert(id, handle);
        Ok(())
    }

    fn ensure_body_slot(&mut self, id: BodyId) -> Result<(), CollisionError> {
        if self.body_handles.contains_key(&id) {
            return Err(CollisionError::BodyAlreadyExists);
        }
        if self.body_handles.len() >= MAX_COLLISION_BODIES_2D {
            return Err(CollisionError::Capacity);
        }

        self.body_handles
            .try_reserve(1)
            .map_err(|_| CollisionError::Capacity)
    }

    fn body_handle(&self, id: BodyId) -> Result<RigidBodyHandle, CollisionError> {
        self.body_handles
            .get(&id)
            .copied()
            .ok_or(CollisionError::InvalidBody)
    }
}
