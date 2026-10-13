//! Public backend acceptance for stored angular velocity and unsupported rotation.

use engine_identifiers::allocate_next;
use engine_physics_api::{BodyId, BodyState, PhysicsError};

use engine_physics_3d::MotionWorld;

fn committed_world() -> (MotionWorld, BodyId) {
    let mut world = MotionWorld::default();
    let body = allocate_next().unwrap().into();
    world.reserve(body, [0.0; 3]).unwrap();
    world.commit(body).unwrap();
    (world, body)
}

fn state_with_angular_velocity(
    world: &MotionWorld,
    body: BodyId,
    angular_velocity: [f32; 3],
) -> BodyState {
    let mut state = world.body(body).unwrap();
    let mut configuration = state.configuration();
    configuration
        .set_angular_velocity(angular_velocity)
        .unwrap();
    state.set_configuration(configuration);
    state
}

#[test]
fn nonzero_angular_motion_is_rejected_without_body_mutation() {
    let (mut world, body) = committed_world();
    let state = state_with_angular_velocity(&world, body, [1.25, -2.0, 3.5]);
    world.set_body(body, state).unwrap();
    let previous_state = world.body(body).unwrap();

    assert_eq!(
        world.prepare_step(0.25, [0.0, -4.0, 0.0]).unwrap_err(),
        PhysicsError::Unsupported
    );
    assert_eq!(world.body(body).unwrap(), previous_state);
}

#[test]
fn successful_angular_write_invalidates_a_plan_before_rejection() {
    let (mut world, body) = committed_world();
    let plan = world.prepare_step(0.25, [0.0, -4.0, 0.0]).unwrap();
    let replacement = state_with_angular_velocity(&world, body, [0.0, 0.0, 2.0]);
    world.set_body(body, replacement).unwrap();

    assert_eq!(world.commit_step(plan), Err(PhysicsError::StalePlan));
    assert_eq!(
        world.prepare_step(0.25, [0.0, -4.0, 0.0]).unwrap_err(),
        PhysicsError::Unsupported
    );
    assert_eq!(world.body(body).unwrap(), replacement);
}

#[test]
fn signed_zero_reset_restores_translation_stepping() {
    let (mut world, body) = committed_world();
    let mut state = state_with_angular_velocity(&world, body, [0.0, 1.0, 0.0]);
    state.set_linear_velocity([2.0, 0.0, -1.0]).unwrap();
    world.set_body(body, state).unwrap();

    let reset = state_with_angular_velocity(&world, body, [-0.0, 0.0, -0.0]);
    world.set_body(body, reset).unwrap();

    let plan = world.prepare_step(0.25, [0.0, -4.0, 0.0]).unwrap();
    world.commit_step(plan).unwrap();

    let stepped = world.body(body).unwrap();
    assert_eq!(stepped.position(), [0.5, -0.25, -0.25]);
    assert_eq!(stepped.linear_velocity(), [2.0, -1.0, -1.0]);
    let angular_velocity = stepped.configuration().angular_velocity();
    assert!(angular_velocity[0].is_sign_negative());
    assert_eq!(angular_velocity[1], 0.0);
    assert!(angular_velocity[2].is_sign_negative());
}
