use serde::Serialize;

use super::arguments::Dimension;
use super::error::PhysicsSceneError;
use super::scene::RuntimeScene;
use super::snapshot::{CaptureMoment, SceneSnapshot, capture_scene};

pub(super) const FIXED_TIMESTEP_SECONDS: f32 = 1.0 / 60.0;

const CONTACT_HEIGHT_TOLERANCE: f32 = 0.01;
const CONTACT_SPEED_TOLERANCE: f32 = 0.05;

/// Evidence inferred from the final native solver pose and vertical velocity.
#[derive(Clone, Copy, Debug, Serialize)]
pub(super) struct CollisionEvidence {
    pub(super) inferred_resting_contact: bool,
    pub(super) inference_method: &'static str,
    pub(super) expected_box_center_y: f32,
    pub(super) final_box_center_y: f32,
    pub(super) final_height_error: f32,
    pub(super) height_tolerance: f32,
    pub(super) final_vertical_speed: f32,
    pub(super) vertical_speed_tolerance: f32,
}

/// The actual initial, middle, and final runtime observations for one scene.
pub(super) struct SimulationReport {
    pub(super) dimension: Dimension,
    pub(super) step_count: usize,
    pub(super) snapshots: [SceneSnapshot; 3],
    pub(super) collision: CollisionEvidence,
}

pub(super) fn run(
    scene: &mut RuntimeScene,
    step_count: usize,
) -> Result<SimulationReport, PhysicsSceneError> {
    let snapshots = capture_timeline(scene, step_count)?;
    let collision = verify_resting_contact(scene, &snapshots)?;

    Ok(SimulationReport {
        dimension: scene.dimension,
        step_count,
        snapshots,
        collision,
    })
}

fn capture_timeline(
    scene: &mut RuntimeScene,
    step_count: usize,
) -> Result<[SceneSnapshot; 3], PhysicsSceneError> {
    let middle_step = step_count / 2;
    let mut snapshots = Vec::with_capacity(3);
    snapshots.push(capture_scene(scene, CaptureMoment::Initial, 0)?);

    for step in 1..=step_count {
        scene
            .runtime
            .physics_scene_step(FIXED_TIMESTEP_SECONDS)
            .map_err(|error| PhysicsSceneError::debug("advance native collision scene", error))?;

        if step == middle_step {
            snapshots.push(capture_scene(scene, CaptureMoment::Middle, step)?);
        }
        if step == step_count {
            snapshots.push(capture_scene(scene, CaptureMoment::Final, step)?);
        }
    }

    snapshots.try_into().map_err(|_| {
        PhysicsSceneError::new("physics timeline did not contain three distinct captures")
    })
}

fn verify_resting_contact(
    scene: &RuntimeScene,
    snapshots: &[SceneSnapshot; 3],
) -> Result<CollisionEvidence, PhysicsSceneError> {
    let initial_dynamic = snapshots[0].bodies[1];
    let final_floor = snapshots[2].bodies[0];
    let final_dynamic = snapshots[2].bodies[1];
    let expected_box_center_y = final_floor.transform.position[1]
        + scene.floor.half_extents[1]
        + scene.dynamic.half_extents[1];
    let final_box_center_y = final_dynamic.transform.position[1];
    let final_height_error = (final_box_center_y - expected_box_center_y).abs();
    let final_vertical_speed = final_dynamic.physics.linear_velocity[1].abs();

    if initial_dynamic.transform.position[1] <= final_box_center_y
        || final_height_error > CONTACT_HEIGHT_TOLERANCE
        || final_vertical_speed > CONTACT_SPEED_TOLERANCE
    {
        return Err(PhysicsSceneError::new(format!(
            "dynamic body did not settle on the fixed floor after {} steps (height error {final_height_error:.3}, vertical speed {final_vertical_speed:.3})",
            snapshots[2].simulation_step
        )));
    }

    Ok(CollisionEvidence {
        inferred_resting_contact: true,
        inference_method: "final solver pose is near the expected floor contact height and vertical speed is bounded; no contact-event API is exposed",
        expected_box_center_y,
        final_box_center_y,
        final_height_error,
        height_tolerance: CONTACT_HEIGHT_TOLERANCE,
        final_vertical_speed,
        vertical_speed_tolerance: CONTACT_SPEED_TOLERANCE,
    })
}
