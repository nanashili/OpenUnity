//! Failed input, overflow and private-plan provenance preserve live world state.

use engine_identifiers::allocate_next;
use engine_physics_3d::MotionWorld;
use engine_physics_api::{BodyId, PhysicsError};

fn populated_world(position: [f32; 3], velocity: [f32; 3]) -> (MotionWorld, BodyId) {
    let mut world = MotionWorld::default();
    let body = allocate_next().unwrap().into();
    world.reserve(body, position).unwrap();
    world.commit(body).unwrap();
    let mut state = world.body(body).unwrap();
    state.set_linear_velocity(velocity).unwrap();
    world.set_body(body, state).unwrap();
    (world, body)
}

#[test]
fn invalid_step_inputs_fail_for_populated_and_empty_worlds_without_mutation() {
    let (world, body) = populated_world([1.0, 2.0, 3.0], [4.0, 5.0, 6.0]);
    let previous = world.body(body).unwrap();
    let empty = MotionWorld::default();

    for duration in [0.0, -0.25, f32::NAN, f32::INFINITY] {
        assert_eq!(
            world.prepare_step(duration, [0.0; 3]).unwrap_err(),
            PhysicsError::Numeric
        );
        assert_eq!(world.body(body).unwrap(), previous);
        assert_eq!(
            empty.prepare_step(duration, [0.0; 3]).unwrap_err(),
            PhysicsError::Numeric
        );
    }
    assert_eq!(
        world
            .prepare_step(0.25, [0.0, 0.0, f32::NEG_INFINITY])
            .unwrap_err(),
        PhysicsError::Numeric
    );
    assert_eq!(world.body(body).unwrap(), previous);
    assert_eq!(
        empty
            .prepare_step(0.25, [0.0, 0.0, f32::NEG_INFINITY])
            .unwrap_err(),
        PhysicsError::Numeric
    );
}

#[test]
fn velocity_and_later_position_overflow_publish_no_partial_motion() {
    let (velocity_world, velocity_id) = populated_world([1.0; 3], [f32::MAX, 0.0, 0.0]);
    let previous_velocity = velocity_world.body(velocity_id).unwrap();
    assert_eq!(
        velocity_world
            .prepare_step(1.0, [f32::MAX, 0.0, 0.0])
            .unwrap_err(),
        PhysicsError::Numeric
    );
    assert_eq!(velocity_world.body(velocity_id).unwrap(), previous_velocity);

    let (position_world, position_id) = populated_world([f32::MAX, 2.0, 3.0], [f32::MAX, 0.0, 0.0]);
    let previous_position = position_world.body(position_id).unwrap();
    // Gravity changes velocity successfully before the later position addition overflows.
    assert_eq!(
        position_world
            .prepare_step(1.0, [0.0, -4.0, 0.0])
            .unwrap_err(),
        PhysicsError::Numeric
    );
    assert_eq!(position_world.body(position_id).unwrap(), previous_position);
}

#[test]
fn real_mutation_and_remove_recreate_invalidate_prepared_steps() {
    let (mut world, body) = populated_world([1.0; 3], [2.0, 0.0, -1.0]);
    let stale = world.prepare_step(0.25, [0.0, -4.0, 0.0]).unwrap();
    let mut replacement = world.body(body).unwrap();
    replacement.set_position([9.0; 3]).unwrap();
    world.set_body(body, replacement).unwrap();
    assert_eq!(world.commit_step(stale), Err(PhysicsError::StalePlan));
    assert_eq!(world.body(body).unwrap(), replacement);

    let stale = world.prepare_step(0.25, [0.0, -4.0, 0.0]).unwrap();
    world.remove(body).unwrap();
    let new_body = allocate_next().unwrap().into();
    world.reserve(new_body, [7.0, 8.0, 9.0]).unwrap();
    world.commit(new_body).unwrap();
    let initial = world.body(new_body).unwrap();
    assert_eq!(world.commit_step(stale), Err(PhysicsError::StalePlan));
    assert_eq!(world.body(new_body).unwrap(), initial);
    assert_eq!(world.body(body), Err(PhysicsError::InvalidBody));

    let current = world.prepare_step(0.25, [0.0, -4.0, 0.0]).unwrap();
    world.commit_step(current).unwrap();
    assert_eq!(world.body(new_body).unwrap().position(), [7.0, 7.75, 9.0]);
    assert_eq!(
        world.body(new_body).unwrap().linear_velocity(),
        [0.0, -1.0, 0.0]
    );
}

#[test]
fn successful_motion_invalidates_an_alternative_prepared_from_the_same_state() {
    let (mut world, body) = populated_world([0.0; 3], [2.0, 0.0, -1.0]);
    let first = world.prepare_step(0.25, [0.0, -4.0, 0.0]).unwrap();
    let alternative = world.prepare_step(0.5, [0.0, -4.0, 0.0]).unwrap();
    world.commit_step(first).unwrap();

    assert_eq!(world.commit_step(alternative), Err(PhysicsError::StalePlan));
    assert_eq!(world.body(body).unwrap().position(), [0.5, -0.25, -0.25]);
    assert_eq!(
        world.body(body).unwrap().linear_velocity(),
        [2.0, -1.0, -1.0]
    );
}

#[test]
fn private_world_provenance_rejects_foreign_plans_even_with_same_identity_and_revision() {
    let (first, body) = populated_world([1.0; 3], [2.0, 0.0, -1.0]);
    let mut second = MotionWorld::default();
    second.reserve(body, [8.0; 3]).unwrap();
    second.commit(body).unwrap();
    let mut previous = second.body(body).unwrap();
    previous.set_linear_velocity([3.0, 0.0, 1.0]).unwrap();
    second.set_body(body, previous).unwrap();

    let foreign = first.prepare_step(0.25, [0.0, -4.0, 0.0]).unwrap();
    assert_eq!(second.commit_step(foreign), Err(PhysicsError::StalePlan));
    assert_eq!(second.body(body).unwrap(), previous);
    assert_eq!(first.body(body).unwrap().position(), [1.0; 3]);
    let current = second.prepare_step(0.25, [0.0, -4.0, 0.0]).unwrap();
    second.commit_step(current).unwrap();
    assert_eq!(second.body(body).unwrap().position(), [8.75, 7.75, 8.25]);
}
