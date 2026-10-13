//! Collision-world ownership and gravity configuration.

use std::collections::HashMap;

use engine_physics_api::{BodyId, CollisionError};
use rapier2d::prelude::{
    BroadPhaseMultiSap, CCDSolver, ColliderSet, ImpulseJointSet, IntegrationParameters,
    IslandManager, MultibodyJointSet, NarrowPhase, PhysicsPipeline, RigidBodyHandle, RigidBodySet,
    Vector,
};

use crate::validation::validate_gravity;

/// A bounded physics world with private Rapier storage and engine identity mapping.
pub struct CollisionWorld2D {
    pub(super) gravity: Vector<f32>,
    pub(super) body_handles: HashMap<BodyId, RigidBodyHandle>,
    pub(super) pipeline: PhysicsPipeline,
    pub(super) integration_parameters: IntegrationParameters,
    pub(super) islands: IslandManager,
    pub(super) broad_phase: BroadPhaseMultiSap,
    pub(super) narrow_phase: NarrowPhase,
    pub(super) rigid_bodies: RigidBodySet,
    pub(super) colliders: ColliderSet,
    pub(super) impulse_joints: ImpulseJointSet,
    pub(super) multibody_joints: MultibodyJointSet,
    pub(super) ccd_solver: CCDSolver,
}

impl Clone for CollisionWorld2D {
    fn clone(&self) -> Self {
        Self {
            gravity: self.gravity,
            body_handles: self.body_handles.clone(),
            pipeline: PhysicsPipeline::new(),
            integration_parameters: self.integration_parameters,
            islands: self.islands.clone(),
            broad_phase: self.broad_phase.clone(),
            narrow_phase: self.narrow_phase.clone(),
            rigid_bodies: self.rigid_bodies.clone(),
            colliders: self.colliders.clone(),
            impulse_joints: self.impulse_joints.clone(),
            multibody_joints: self.multibody_joints.clone(),
            ccd_solver: self.ccd_solver.clone(),
        }
    }
}

impl CollisionWorld2D {
    /// Create an empty world with explicit finite gravity.
    ///
    /// # Errors
    /// Returns InvalidNumber when a gravity component is nonfinite or exceeds
    /// the supported magnitude.
    pub fn new(gravity: [f32; 2]) -> Result<Self, CollisionError> {
        validate_gravity(gravity)?;

        Ok(Self {
            gravity: Vector::new(gravity[0], gravity[1]),
            body_handles: HashMap::new(),
            pipeline: PhysicsPipeline::new(),
            integration_parameters: IntegrationParameters::default(),
            islands: IslandManager::new(),
            broad_phase: BroadPhaseMultiSap::new(),
            narrow_phase: NarrowPhase::new(),
            rigid_bodies: RigidBodySet::new(),
            colliders: ColliderSet::new(),
            impulse_joints: ImpulseJointSet::new(),
            multibody_joints: MultibodyJointSet::new(),
            ccd_solver: CCDSolver::new(),
        })
    }

    /// Return the world's copied gravity vector.
    pub fn gravity(&self) -> [f32; 2] {
        [self.gravity.x, self.gravity.y]
    }

    /// Replace gravity after validating both components.
    ///
    /// # Errors
    /// Returns InvalidNumber when a gravity component is nonfinite or exceeds
    /// the supported magnitude.
    pub fn set_gravity(&mut self, gravity: [f32; 2]) -> Result<(), CollisionError> {
        validate_gravity(gravity)?;
        self.gravity = Vector::new(gravity[0], gravity[1]);
        Ok(())
    }
}
