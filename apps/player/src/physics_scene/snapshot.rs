use engine_math::{Quaternion, Vector};
use serde::Serialize;

use super::error::PhysicsSceneError;
use super::scene::{BodyDefinition, BodyRole, RuntimeScene};

const POSE_TOLERANCE: f32 = 0.0001;

/// Which bounded point in the physics step sequence produced a snapshot.
#[derive(Clone, Copy, Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum CaptureMoment {
    Initial,
    Middle,
    Final,
    Live,
}

/// Actual local Transform values read from the live runtime.
#[derive(Clone, Copy, Debug, Serialize)]
pub(super) struct TransformSnapshot {
    pub(super) position: Vector,
    pub(super) rotation: Quaternion,
    pub(super) scale: Vector,
}

/// Solver state copied through the runtime's dimension-neutral API.
#[derive(Clone, Copy, Debug, Serialize)]
pub(super) struct PhysicsSnapshot {
    pub(super) position: Vector,
    pub(super) rotation: Quaternion,
    pub(super) linear_velocity: Vector,
    pub(super) dynamic: bool,
}

/// Transform and solver facts for one scene body at one simulation step.
#[derive(Clone, Copy, Debug, Serialize)]
pub(super) struct BodySnapshot {
    pub(super) role: BodyRole,
    pub(super) owner_id: i64,
    pub(super) collider_half_extents: Vector,
    pub(super) mesh_index: u32,
    pub(super) material_index: u32,
    pub(super) transform: TransformSnapshot,
    pub(super) physics: PhysicsSnapshot,
}

/// A renderable native scene observation at a specific simulation step.
#[derive(Clone, Copy, Debug, Serialize)]
pub(super) struct SceneSnapshot {
    pub(super) moment: CaptureMoment,
    pub(super) simulation_step: usize,
    pub(super) bodies: [BodySnapshot; 2],
}

/// Final state exposed by the live player route after presentation stops.
#[derive(Clone, Copy, Debug, Serialize)]
pub(crate) struct DynamicBodyState {
    pub(crate) transform_position: Vector,
    pub(crate) solver_position: Vector,
    pub(crate) solver_rotation: Quaternion,
    pub(crate) linear_velocity: Vector,
    pub(crate) is_dynamic: bool,
}

pub(super) fn capture_scene(
    scene: &RuntimeScene,
    moment: CaptureMoment,
    simulation_step: usize,
) -> Result<SceneSnapshot, PhysicsSceneError> {
    let floor = capture_body(&scene.runtime, scene.floor)?;
    let dynamic = capture_body(&scene.runtime, scene.dynamic)?;

    Ok(SceneSnapshot {
        moment,
        simulation_step,
        bodies: [floor, dynamic],
    })
}

pub(crate) fn capture_dynamic_body_state(
    scene: &RuntimeScene,
    simulation_step: usize,
) -> Result<DynamicBodyState, PhysicsSceneError> {
    let snapshot = capture_scene(scene, CaptureMoment::Live, simulation_step)?;
    let dynamic = snapshot.bodies[1];

    Ok(DynamicBodyState {
        transform_position: dynamic.transform.position,
        solver_position: dynamic.physics.position,
        solver_rotation: dynamic.physics.rotation,
        linear_velocity: dynamic.physics.linear_velocity,
        is_dynamic: dynamic.physics.dynamic,
    })
}

fn capture_body(
    runtime: &engine_runtime::Runtime,
    definition: BodyDefinition,
) -> Result<BodySnapshot, PhysicsSceneError> {
    let transform = TransformSnapshot {
        position: read_property(runtime, definition.transform, 1)?,
        rotation: read_property(runtime, definition.transform, 2)?,
        scale: read_property(runtime, definition.transform, 3)?,
    };
    let solver = runtime
        .physics_scene_body_state(definition.owner)
        .map_err(|error| PhysicsSceneError::debug("read native collision body state", error))?;
    let physics = PhysicsSnapshot {
        position: solver.position(),
        rotation: solver.rotation(),
        linear_velocity: solver.linear_velocity(),
        dynamic: solver.is_dynamic(),
    };

    ensure_body_kind(definition.role, physics.dynamic)?;
    ensure_pose_published(definition.role, transform, physics)?;

    Ok(BodySnapshot {
        role: definition.role,
        owner_id: definition.owner.to_raw(),
        collider_half_extents: definition.half_extents,
        mesh_index: definition.mesh_index,
        material_index: definition.material_index,
        transform,
        physics,
    })
}

fn ensure_body_kind(role: BodyRole, dynamic: bool) -> Result<(), PhysicsSceneError> {
    let expected_dynamic = matches!(role, BodyRole::DynamicBox);
    if dynamic != expected_dynamic {
        return Err(PhysicsSceneError::new(format!(
            "native solver returned an unexpected body kind for {role:?}"
        )));
    }

    Ok(())
}

fn read_property<const LENGTH: usize>(
    runtime: &engine_runtime::Runtime,
    transform: engine_object::ObjectId,
    property: i32,
) -> Result<[f32; LENGTH], PhysicsSceneError> {
    let values = runtime
        .transform_read(transform, property)
        .map_err(|error| PhysicsSceneError::debug("read runtime Transform snapshot", error))?;

    values.try_into().map_err(|_| {
        PhysicsSceneError::new(format!(
            "Transform property {property} returned an unexpected component count"
        ))
    })
}

fn ensure_pose_published(
    role: BodyRole,
    transform: TransformSnapshot,
    physics: PhysicsSnapshot,
) -> Result<(), PhysicsSceneError> {
    ensure_unit_root_scale(role, transform.scale)?;

    let position_matches = transform
        .position
        .iter()
        .zip(physics.position)
        .all(|(actual, expected)| (actual - expected).abs() <= POSE_TOLERANCE);
    let rotation_matches =
        quaternion_alignment(transform.rotation, physics.rotation) >= 1.0 - POSE_TOLERANCE;

    if !position_matches || !rotation_matches {
        return Err(PhysicsSceneError::new(format!(
            "runtime Transform for {role:?} does not match its published solver pose"
        )));
    }

    Ok(())
}

fn ensure_unit_root_scale(role: BodyRole, scale: Vector) -> Result<(), PhysicsSceneError> {
    if scale != [1.0, 1.0, 1.0] {
        return Err(PhysicsSceneError::new(format!(
            "runtime Transform for {role:?} does not retain unit root scale"
        )));
    }

    Ok(())
}

fn quaternion_alignment(left: Quaternion, right: Quaternion) -> f32 {
    left.iter()
        .zip(right)
        .map(|(left, right)| left * right)
        .sum::<f32>()
        .abs()
}
