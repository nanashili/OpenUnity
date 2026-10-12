//! Runtime composition of domain objects, private hierarchy poses, and scheduling.

use crate::{
    lifecycle,
    math::{Quaternion, Vector},
    scene, timing,
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
#[derive(Debug, Default)]
pub struct Runtime {
    pub(super) objects: ObjectStore,
    pub(super) transforms: HashMap<ObjectId, TransformData>,
    pub(super) lifecycle: lifecycle::Lifecycle,
    pub(super) scenes: scene::Scenes,
    pub(super) clock: timing::Clock,
    pub(super) gravity: GravityConfiguration,
}
