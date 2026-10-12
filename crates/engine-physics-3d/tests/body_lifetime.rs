//! Public world acceptance of reservation, publication, retirement and isolation.

use engine_identifiers::allocate_next;
use engine_physics_3d::MotionWorld;
use engine_physics_api::{BodyId, BodyState, PhysicsError};

fn fresh_body() -> BodyId {
    allocate_next().unwrap().into()
}

#[test]
fn reservations_are_hidden_and_abort_does_not_publish_or_occupy_capacity() {
    let mut world = MotionWorld::default();
    let reserved = fresh_body();
    let next = fresh_body();
    world.reserve(reserved, [1.0, 2.0, 3.0]).unwrap();
    assert_eq!(world.body(reserved), Err(PhysicsError::InvalidState));
    assert_eq!(world.remove(reserved), Err(PhysicsError::InvalidState));
    assert_eq!(
        world.reserve(next, [4.0, 5.0, 6.0]),
        Err(PhysicsError::Capacity)
    );
    assert_eq!(world.abort(next), Err(PhysicsError::InvalidBody));

    let step = world.prepare_step(0.25, [0.0, -4.0, 0.0]).unwrap();
    assert!(step.body().is_none());
    world.commit_step(step).unwrap();
    assert_eq!(world.body(reserved), Err(PhysicsError::InvalidState));
    world.abort(reserved).unwrap();
    assert_eq!(world.body(reserved), Err(PhysicsError::InvalidBody));

    world.reserve(next, [4.0, 5.0, 6.0]).unwrap();
    world.commit(next).unwrap();
    assert_eq!(world.body(next).unwrap().position(), [4.0, 5.0, 6.0]);
    assert_eq!(world.body(next).unwrap().linear_velocity(), [0.0; 3]);
}

#[test]
fn committed_body_requires_matching_identity_and_retirement_invalidates_reads() {
    let mut world = MotionWorld::default();
    let body = fresh_body();
    let other = fresh_body();
    world.reserve(body, [1.0, 2.0, 3.0]).unwrap();
    assert_eq!(world.commit(other), Err(PhysicsError::InvalidBody));
    world.commit(body).unwrap();
    let before = world.body(body).unwrap();

    assert_eq!(world.commit(body), Err(PhysicsError::InvalidState));
    assert_eq!(world.abort(body), Err(PhysicsError::InvalidState));
    assert_eq!(world.reserve(other, [0.0; 3]), Err(PhysicsError::Capacity));
    assert_eq!(
        world.set_body(other, BodyState::new([9.0; 3]).unwrap()),
        Err(PhysicsError::InvalidBody)
    );
    assert_eq!(world.remove(other), Err(PhysicsError::InvalidBody));
    assert_eq!(world.body(body).unwrap(), before);

    world.remove(body).unwrap();
    assert_eq!(world.body(body), Err(PhysicsError::InvalidBody));
    assert_eq!(world.remove(body), Err(PhysicsError::InvalidBody));
    world.reserve(other, [7.0, 8.0, 9.0]).unwrap();
    world.commit(other).unwrap();
    assert_eq!(world.body(body), Err(PhysicsError::InvalidBody));
    assert_eq!(world.body(other).unwrap().position(), [7.0, 8.0, 9.0]);
}

#[test]
fn separate_worlds_own_separate_copies_and_invalid_creation_leaves_capacity_free() {
    let mut first = MotionWorld::default();
    let mut second = MotionWorld::default();
    let first_id = fresh_body();
    let second_id = fresh_body();
    assert_eq!(
        first.reserve(first_id, [0.0, 0.0, f32::NAN]),
        Err(PhysicsError::Numeric)
    );
    first.reserve(first_id, [1.0; 3]).unwrap();
    first.commit(first_id).unwrap();
    second.reserve(second_id, [2.0; 3]).unwrap();
    second.commit(second_id).unwrap();

    let mut copy = first.body(first_id).unwrap();
    copy.set_position([3.0; 3]).unwrap();
    assert_eq!(first.body(first_id).unwrap().position(), [1.0; 3]);
    first.set_body(first_id, copy).unwrap();
    assert_eq!(first.body(first_id).unwrap().position(), [3.0; 3]);
    assert_eq!(second.body(second_id).unwrap().position(), [2.0; 3]);
    assert_eq!(first.body(second_id), Err(PhysicsError::InvalidBody));
    assert_eq!(second.body(first_id), Err(PhysicsError::InvalidBody));
}
