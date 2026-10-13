//! Public backend acceptance for mixed reservation/commit capacity.
//!
//! The probe uses only MotionWorld's public API. It checks that a failed
//! over-capacity reservation preserves both stored states and prepared plans.

use engine_identifiers::allocate_next;
use engine_physics_3d::{MAX_MOTION_BODIES, MotionWorld};
use engine_physics_api::{BodyId, PhysicsError};

fn fresh_body() -> BodyId {
    allocate_next().expect("allocate process identity").into()
}

#[test]
fn mixed_capacity_rejection_retains_state_and_releases_only_the_aborted_slot() {
    let mut world = MotionWorld::default();
    let mut bodies = Vec::with_capacity(MAX_MOTION_BODIES);
    let mut committed = Vec::with_capacity(MAX_MOTION_BODIES / 2);
    let mut reserved = Vec::with_capacity(MAX_MOTION_BODIES / 2);

    for index in 0..MAX_MOTION_BODIES {
        let body = fresh_body();
        world
            .reserve(body, [index as f32, 1.0, -1.0])
            .expect("reserve within the exact world capacity");
        if index % 2 == 0 {
            world.commit(body).expect("publish alternating body");
            committed.push(body);
        } else {
            reserved.push(body);
        }
        bodies.push(body);
    }

    let before = world.bodies().collect::<Vec<_>>();
    let step_before_rejection = world
        .prepare_step(0.25, [0.0, -4.0, 0.0])
        .expect("prepare a plan at capacity");
    let rejected_id = fresh_body();

    assert_eq!(
        world.reserve(rejected_id, [9.0, 8.0, 7.0]),
        Err(PhysicsError::Capacity)
    );
    assert_eq!(
        world.reserve(fresh_body(), [f32::NAN, 0.0, 0.0]),
        Err(PhysicsError::Capacity),
        "capacity keeps precedence over invalid pose"
    );
    assert_eq!(world.bodies().collect::<Vec<_>>(), before);
    for body in committed {
        assert!(world.body(body).is_ok());
    }
    for body in reserved {
        assert_eq!(world.body(body), Err(PhysicsError::InvalidState));
    }
    assert_eq!(world.body(rejected_id), Err(PhysicsError::InvalidBody));

    // A plan remains committable only if the failed reserve left revision intact.
    world.commit_step(step_before_rejection).unwrap();

    let released_reservation = bodies[1];
    world.abort(released_reservation).unwrap();
    let replacement = fresh_body();
    world
        .reserve(replacement, [3.0, 4.0, 5.0])
        .expect("one exact abort releases one slot");

    let next_rejected = fresh_body();
    assert_eq!(
        world.reserve(next_rejected, [0.0; 3]),
        Err(PhysicsError::Capacity)
    );
    assert_eq!(world.body(replacement), Err(PhysicsError::InvalidState));
}

#[test]
fn mixed_batch_retirement_validates_every_id_before_removal() {
    let mut world = MotionWorld::default();
    let lower = fresh_body();
    let higher = fresh_body();
    let reserved = fresh_body();
    let missing = fresh_body();

    world.reserve(lower, [1.0, 2.0, 3.0]).unwrap();
    world.commit(lower).unwrap();
    world.reserve(higher, [4.0, 5.0, 6.0]).unwrap();
    world.commit(higher).unwrap();
    world.reserve(reserved, [7.0, 8.0, 9.0]).unwrap();

    let lower_before = world.body(lower).unwrap();
    let higher_before = world.body(higher).unwrap();
    let missing_plan = world.prepare_step(0.25, [0.0; 3]).unwrap();
    assert_eq!(
        world.retire_bodies(&[lower, missing]),
        Err(PhysicsError::InvalidBody)
    );
    assert_eq!(world.body(lower), Ok(lower_before));
    assert_eq!(world.body(higher), Ok(higher_before));
    assert_eq!(world.body(reserved), Err(PhysicsError::InvalidState));
    world.commit_step(missing_plan).unwrap();

    let duplicate_plan = world.prepare_step(0.25, [0.0; 3]).unwrap();
    assert_eq!(
        world.retire_bodies(&[lower, lower]),
        Err(PhysicsError::InvalidState)
    );
    assert_eq!(world.body(lower), Ok(lower_before));
    assert_eq!(world.body(higher), Ok(higher_before));
    assert_eq!(world.body(reserved), Err(PhysicsError::InvalidState));
    world.commit_step(duplicate_plan).unwrap();

    let stale_plan = world.prepare_step(0.25, [0.0; 3]).unwrap();
    world.retire_bodies(&[lower, reserved]).unwrap();
    assert_eq!(world.body(lower), Err(PhysicsError::InvalidBody));
    assert_eq!(world.body(reserved), Err(PhysicsError::InvalidBody));
    assert_eq!(world.body(higher), Ok(higher_before));
    assert_eq!(world.commit_step(stale_plan), Err(PhysicsError::StalePlan));

    let empty_batch_plan = world.prepare_step(0.25, [0.0; 3]).unwrap();
    world.retire_bodies(&[]).unwrap();
    world.commit_step(empty_batch_plan).unwrap();
}
