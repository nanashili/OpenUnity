use engine_math::{Quaternion, Vector, finite};
use engine_physics_api::{BodyId, CollisionError};
use rapier3d::na::{
    Isometry3, Quaternion as RapierQuaternion, Translation3, UnitQuaternion, Vector3,
};
use rapier3d::prelude::{
    BroadPhaseMultiSap, CCDSolver, ColliderBuilder, ColliderSet, ImpulseJointSet,
    IntegrationParameters, IslandManager, MultibodyJointSet, NarrowPhase, PhysicsPipeline,
    RigidBodyBuilder, RigidBodyHandle, RigidBodySet,
};

use super::{CollisionBodySnapshot, MAX_ABSOLUTE_POSITION_COMPONENT, MAX_COLLISION_BODIES};

#[derive(Clone, Copy)]
struct BodyRecord {
    id: BodyId,
    handle: RigidBodyHandle,
}

#[derive(Clone, Default)]
pub(super) struct SolverState {
    islands: IslandManager,
    broad_phase: BroadPhaseMultiSap,
    narrow_phase: NarrowPhase,
    bodies: RigidBodySet,
    colliders: ColliderSet,
    impulse_joints: ImpulseJointSet,
    multibody_joints: MultibodyJointSet,
    ccd_solver: CCDSolver,
    body_records: Vec<BodyRecord>,
}

impl SolverState {
    pub(super) fn add_cuboid(
        &mut self,
        id: BodyId,
        builder: RigidBodyBuilder,
        position: Vector,
        rotation: Quaternion,
        half_extents: Vector,
    ) -> Result<(), CollisionError> {
        self.validate_insert(id)?;
        self.reserve_body_record()?;

        let handle = self.insert_rapier_cuboid(builder, position, rotation, half_extents);
        self.body_records.push(BodyRecord { id, handle });
        Ok(())
    }

    pub(super) fn body(&self, id: BodyId) -> Result<CollisionBodySnapshot, CollisionError> {
        let record = self.body_record(id)?;
        self.snapshot(record.handle)
    }

    pub(super) fn remove(&mut self, id: BodyId) -> Result<(), CollisionError> {
        let record_index = self.body_record_index(id)?;
        let record = self.body_records[record_index];
        let mut candidate = self.clone();

        candidate.remove_rapier_body(record.handle)?;
        candidate.body_records.remove(record_index);

        *self = candidate;
        Ok(())
    }

    pub(super) fn step(&mut self, gravity: Vector, timestep: f32) -> Result<(), CollisionError> {
        let mut candidate = self.clone();
        candidate.advance(gravity, timestep);
        candidate.validate_all_snapshots()?;

        *self = candidate;
        Ok(())
    }

    pub(super) fn body_count(&self) -> usize {
        self.body_records.len()
    }

    fn validate_insert(&self, id: BodyId) -> Result<(), CollisionError> {
        if self.body_records.iter().any(|record| record.id == id) {
            return Err(CollisionError::BodyAlreadyExists);
        }
        if self.body_records.len() >= MAX_COLLISION_BODIES {
            return Err(CollisionError::Capacity);
        }
        Ok(())
    }

    fn reserve_body_record(&mut self) -> Result<(), CollisionError> {
        self.body_records
            .try_reserve(1)
            .map_err(|_| CollisionError::Capacity)
    }

    fn insert_rapier_cuboid(
        &mut self,
        builder: RigidBodyBuilder,
        position: Vector,
        rotation: Quaternion,
        half_extents: Vector,
    ) -> RigidBodyHandle {
        let transform = to_isometry(position, rotation);
        let handle = self.bodies.insert(builder.position(transform).build());
        let collider =
            ColliderBuilder::cuboid(half_extents[0], half_extents[1], half_extents[2]).build();

        self.colliders
            .insert_with_parent(collider, handle, &mut self.bodies);
        handle
    }

    fn remove_rapier_body(&mut self, handle: RigidBodyHandle) -> Result<(), CollisionError> {
        self.bodies
            .remove(
                handle,
                &mut self.islands,
                &mut self.colliders,
                &mut self.impulse_joints,
                &mut self.multibody_joints,
                true,
            )
            .ok_or(CollisionError::InvalidBody)?;

        Ok(())
    }

    fn advance(&mut self, gravity: Vector, timestep: f32) {
        let mut pipeline = PhysicsPipeline::new();
        let parameters = IntegrationParameters {
            dt: timestep,
            ..IntegrationParameters::default()
        };

        pipeline.step(
            &to_rapier_vector(gravity),
            &parameters,
            &mut self.islands,
            &mut self.broad_phase,
            &mut self.narrow_phase,
            &mut self.bodies,
            &mut self.colliders,
            &mut self.impulse_joints,
            &mut self.multibody_joints,
            &mut self.ccd_solver,
            None,
            &(),
            &(),
        );
    }

    fn validate_all_snapshots(&self) -> Result<(), CollisionError> {
        for record in &self.body_records {
            self.snapshot(record.handle)?;
        }
        Ok(())
    }

    fn snapshot(&self, handle: RigidBodyHandle) -> Result<CollisionBodySnapshot, CollisionError> {
        let body = self.bodies.get(handle).ok_or(CollisionError::InvalidBody)?;
        let translation = body.translation();
        let rotation = body.rotation().quaternion();
        let velocity = body.linvel();
        let snapshot = CollisionBodySnapshot {
            position: [translation.x, translation.y, translation.z],
            rotation: [rotation.i, rotation.j, rotation.k, rotation.w],
            linear_velocity: [velocity.x, velocity.y, velocity.z],
        };

        validate_snapshot(snapshot)?;
        Ok(snapshot)
    }

    fn body_record(&self, id: BodyId) -> Result<BodyRecord, CollisionError> {
        let index = self.body_record_index(id)?;
        Ok(self.body_records[index])
    }

    fn body_record_index(&self, id: BodyId) -> Result<usize, CollisionError> {
        self.body_records
            .iter()
            .position(|record| record.id == id)
            .ok_or(CollisionError::InvalidBody)
    }
}

fn to_isometry(position: Vector, rotation: Quaternion) -> Isometry3<f32> {
    let translation = Translation3::new(position[0], position[1], position[2]);
    let rotation = UnitQuaternion::new_normalize(RapierQuaternion::new(
        rotation[3],
        rotation[0],
        rotation[1],
        rotation[2],
    ));

    Isometry3::from_parts(translation, rotation)
}

fn to_rapier_vector(vector: Vector) -> Vector3<f32> {
    Vector3::new(vector[0], vector[1], vector[2])
}

fn validate_snapshot(snapshot: CollisionBodySnapshot) -> Result<(), CollisionError> {
    finite(&snapshot.position).map_err(|_| CollisionError::InvalidNumber)?;
    finite(&snapshot.linear_velocity).map_err(|_| CollisionError::InvalidNumber)?;

    if snapshot
        .position
        .iter()
        .any(|component| component.abs() > MAX_ABSOLUTE_POSITION_COMPONENT)
    {
        return Err(CollisionError::InvalidNumber);
    }

    if snapshot.rotation.iter().all(|value| value.is_finite()) {
        Ok(())
    } else {
        Err(CollisionError::InvalidNumber)
    }
}
