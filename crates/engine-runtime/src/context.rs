//! Runtime composition of domain objects, private hierarchy poses, and scheduling.

use crate::physics;
use crate::{
    input, lifecycle,
    math::{Quaternion, Vector},
    physics_scene, scene, timing,
};
use engine_object::{ObjectId, ObjectStore};
use engine_physics_api::GravityConfiguration;
use std::collections::HashMap;

#[derive(Clone, Debug)]
pub(super) struct TransformData {
    pub(super) owner: ObjectId,
    pub(super) parent: Option<ObjectId>,
    pub(super) children: Vec<ObjectId>,
    pub(super) position: Vector,
    pub(super) rotation: Quaternion,
    pub(super) scale: Vector,
}

/// Owns the identities of one engine context. Dropping it releases its objects.
#[derive(Debug)]
pub struct Runtime {
    pub(super) objects: ObjectStore,
    pub(super) transforms: HashMap<ObjectId, TransformData>,
    pub(super) lifecycle: lifecycle::Lifecycle,
    pub(super) input: input::InputState,
    pub(super) scenes: scene::Scenes,
    pub(super) clock: timing::Clock,
    pub(super) gravity: GravityConfiguration,
    pub(super) physics: physics::PhysicsState,
    pub(super) collision_scene: physics_scene::CollisionSceneState,
}

impl Default for Runtime {
    fn default() -> Self {
        Self {
            objects: ObjectStore::default(),
            transforms: HashMap::new(),
            lifecycle: lifecycle::Lifecycle::default(),
            input: input::InputState::new(),
            scenes: scene::Scenes::default(),
            clock: timing::Clock::default(),
            gravity: GravityConfiguration::default(),
            physics: physics::PhysicsState::default(),
            collision_scene: physics_scene::CollisionSceneState::default(),
        }
    }
}
