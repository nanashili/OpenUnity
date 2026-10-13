use std::collections::BTreeMap;

use engine_identifiers::ObjectId;
use engine_math::Quaternion;
use engine_physics_2d::CollisionWorld2D;
use engine_physics_3d::CollisionWorld3D;
use engine_physics_api::BodyId;

/// Transform link retained with each opaque solver body identity.
#[derive(Clone, Copy, Debug)]
pub(super) struct CollisionBodyBinding {
    pub(super) transform: ObjectId,
    pub(super) original_z: f32,
    pub(super) dynamic: bool,
}

/// At most one explicitly selected collision profile is active per Runtime.
#[derive(Clone, Default)]
pub(crate) struct CollisionSceneState {
    pub(super) backend: Option<CollisionBackend>,
}

#[derive(Clone)]
pub(super) enum CollisionBackend {
    TwoD {
        world: Box<CollisionWorld2D>,
        bodies: BTreeMap<i64, CollisionBodyBinding>,
    },
    ThreeD {
        world: Box<CollisionWorld3D>,
        bodies: BTreeMap<i64, CollisionBodyBinding>,
    },
}

impl CollisionSceneState {
    pub(super) fn dimension(&self) -> Option<super::PhysicsSceneDimension> {
        match self.backend.as_ref()? {
            CollisionBackend::TwoD { .. } => Some(super::PhysicsSceneDimension::TwoD),
            CollisionBackend::ThreeD { .. } => Some(super::PhysicsSceneDimension::ThreeD),
        }
    }

    pub(super) fn body_count(&self) -> usize {
        match self.backend.as_ref() {
            Some(CollisionBackend::TwoD { bodies, .. })
            | Some(CollisionBackend::ThreeD { bodies, .. }) => bodies.len(),
            None => 0,
        }
    }

    pub(super) fn body_state(
        &self,
        owner: ObjectId,
    ) -> Result<super::PhysicsSceneBodyState, crate::RuntimeError> {
        let body_id = BodyId::from(owner);
        let key = owner.to_raw();

        match self
            .backend
            .as_ref()
            .ok_or(crate::RuntimeError::Lifecycle)?
        {
            CollisionBackend::TwoD { world, bodies } => {
                let binding = bodies.get(&key).ok_or(crate::RuntimeError::InvalidObject)?;
                let body = world.body(body_id)?;
                Ok(super::PhysicsSceneBodyState::new(
                    [body.position()[0], body.position()[1], binding.original_z],
                    planar_rotation(body.rotation_radians()),
                    [body.linear_velocity()[0], body.linear_velocity()[1], 0.0],
                    binding.dynamic,
                ))
            }
            CollisionBackend::ThreeD { world, bodies } => {
                let binding = bodies.get(&key).ok_or(crate::RuntimeError::InvalidObject)?;
                let body = world.body(body_id)?;
                Ok(super::PhysicsSceneBodyState::new(
                    body.position(),
                    body.rotation(),
                    body.linear_velocity(),
                    binding.dynamic,
                ))
            }
        }
    }

    pub(super) fn binding(
        &self,
        owner: ObjectId,
    ) -> Result<CollisionBodyBinding, crate::RuntimeError> {
        let bodies = match self
            .backend
            .as_ref()
            .ok_or(crate::RuntimeError::Lifecycle)?
        {
            CollisionBackend::TwoD { bodies, .. } | CollisionBackend::ThreeD { bodies, .. } => {
                bodies
            }
        };
        bodies
            .get(&owner.to_raw())
            .copied()
            .ok_or(crate::RuntimeError::InvalidObject)
    }

    pub(crate) fn contains_owner(&self, owner: ObjectId) -> bool {
        match self.backend.as_ref() {
            Some(CollisionBackend::TwoD { bodies, .. })
            | Some(CollisionBackend::ThreeD { bodies, .. }) => bodies.contains_key(&owner.to_raw()),
            None => false,
        }
    }

    pub(crate) fn contains_transform(&self, transform: ObjectId) -> bool {
        let bodies = match self.backend.as_ref() {
            Some(CollisionBackend::TwoD { bodies, .. })
            | Some(CollisionBackend::ThreeD { bodies, .. }) => bodies,
            None => return false,
        };
        bodies
            .values()
            .any(|binding| binding.transform == transform)
    }
}

impl std::fmt::Debug for CollisionSceneState {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("CollisionSceneState")
            .field("dimension", &self.dimension())
            .field("body_count", &self.body_count())
            .finish()
    }
}

pub(super) fn planar_rotation(angle: f32) -> Quaternion {
    let half_angle = angle * 0.5;
    [0.0, 0.0, half_angle.sin(), half_angle.cos()]
}
