//! Public Runtime transaction acceptance for independently designed multi-body motion.
//!
//! These cases validate OpenUnity policy through safe Runtime calls. They do
//! not extend the accepted single-body Unity reference evidence.

use engine_identifiers::ObjectId;
use engine_runtime::{Runtime, RuntimeError, RuntimeInputError};

#[derive(Clone, Copy)]
struct BodyFixture {
    owner: ObjectId,
    transform: ObjectId,
    body: ObjectId,
}

fn create_body(runtime: &mut Runtime, name: &str) -> BodyFixture {
    let owner = runtime.create_gameobject(name).unwrap();
    let transform = runtime.gameobject_transform(owner).unwrap();
    let body = runtime.rigidbody_reserve(owner).unwrap();
    runtime.rigidbody_commit(body).unwrap();
    BodyFixture {
        owner,
        transform,
        body,
    }
}

fn drain_timed_phase(runtime: &mut Runtime) {
    while let Some(delivery) = runtime.time_next(1).unwrap() {
        runtime.lifecycle_ack(delivery.token, 0).unwrap();
    }
}

fn assert_same_time(runtime: &Runtime, before: engine_runtime::TimeSnapshot) {
    let after = runtime.time_snapshot().unwrap();
    assert_eq!(after.values, before.values);
    assert_eq!(after.frame, before.frame);
    assert_eq!(after.fixed_context, before.fixed_context);
}

#[test]
fn highest_id_overflow_preserves_every_body_and_transform() {
    let mut runtime = Runtime::default();
    let lower = create_body(&mut runtime, "lower-overflow-owner");
    let higher = create_body(&mut runtime, "higher-overflow-owner");

    assert!(lower.body.to_raw() < higher.body.to_raw());
    runtime.physics_mode_write(1).unwrap();
    runtime
        .rigidbody_set_use_gravity(lower.body, false)
        .unwrap();
    runtime
        .rigidbody_set_linear_velocity(lower.body, [1.0, 0.0, 0.0])
        .unwrap();
    runtime
        .rigidbody_set_use_gravity(higher.body, true)
        .unwrap();
    runtime.set_physics_gravity([f32::MAX, 0.0, 0.0]).unwrap();

    let lower_state = runtime.rigidbody_state(lower.body).unwrap();
    let higher_state = runtime.rigidbody_state(higher.body).unwrap();
    let lower_pose = runtime.transform_read(lower.transform, 1).unwrap();
    let higher_pose = runtime.transform_read(higher.transform, 1).unwrap();
    let time_before = runtime.time_snapshot().unwrap();

    assert_eq!(runtime.physics_simulate(2.0), Err(RuntimeError::Numeric));
    assert_same_time(&runtime, time_before);
    assert_eq!(runtime.rigidbody_state(lower.body).unwrap(), lower_state);
    assert_eq!(runtime.rigidbody_state(higher.body).unwrap(), higher_state);
    assert_eq!(
        runtime.transform_read(lower.transform, 1).unwrap(),
        lower_pose
    );
    assert_eq!(
        runtime.transform_read(higher.transform, 1).unwrap(),
        higher_pose
    );

    runtime.set_physics_gravity([0.0; 3]).unwrap();
    runtime.physics_simulate(2.0).unwrap();
    assert_same_time(&runtime, time_before);
    assert_eq!(
        runtime.rigidbody_state(lower.body).unwrap().position(),
        [2.0, 0.0, 0.0]
    );
    assert_eq!(runtime.rigidbody_state(higher.body).unwrap(), higher_state);
}

#[test]
fn failed_sync_preserves_earlier_dirty_body_for_retry() {
    let mut runtime = Runtime::default();
    let first = create_body(&mut runtime, "first-dirty-owner");
    let later = create_body(&mut runtime, "later-pending-owner");
    runtime
        .transform_write(first.transform, 1, &[3.0, 0.0, 0.0])
        .unwrap();
    runtime
        .transform_write(later.transform, 1, &[-2.0, 0.0, 0.0])
        .unwrap();

    let first_before = runtime.rigidbody_state(first.body).unwrap();
    let later_before = runtime.rigidbody_state(later.body).unwrap();
    let first_pose_before = runtime.transform_read(first.transform, 1).unwrap();
    let later_pose_before = runtime.transform_read(later.transform, 1).unwrap();
    runtime.object_destroy_deferred(later.owner).unwrap();

    assert_eq!(
        runtime.physics_sync_transforms(),
        Err(RuntimeError::Lifecycle)
    );
    assert_eq!(runtime.rigidbody_state(first.body).unwrap(), first_before);
    assert_eq!(runtime.rigidbody_state(later.body).unwrap(), later_before);
    assert_eq!(
        runtime.transform_read(first.transform, 1).unwrap(),
        first_pose_before
    );
    assert_eq!(
        runtime.transform_read(later.transform, 1).unwrap(),
        later_pose_before
    );

    runtime.destroy_typed_object(later.owner).unwrap();
    runtime.physics_sync_transforms().unwrap();
    assert_eq!(
        runtime.rigidbody_state(first.body).unwrap().position(),
        [3.0, 0.0, 0.0]
    );
    assert_eq!(
        runtime.transform_read(first.transform, 1).unwrap(),
        vec![3.0, 0.0, 0.0]
    );
}

#[test]
fn fixed_mode_rejects_two_body_frame_and_due_step_without_time_mutation() {
    let mut runtime = Runtime::default();
    create_body(&mut runtime, "first-fixed-owner");
    create_body(&mut runtime, "second-fixed-owner");
    runtime.physics_mode_write(0).unwrap();

    assert!(matches!(
        runtime.lifecycle_begin_frame(),
        Err(RuntimeInputError::Runtime(RuntimeError::PhysicsUnsupported))
    ));
    assert_eq!(runtime.lifecycle_frame_count(), 0);

    runtime.time_configure(1, 0.02).unwrap();
    let initial_time = runtime.time_snapshot().unwrap();
    assert!(matches!(
        runtime.time_begin_frame(),
        Err(RuntimeInputError::Runtime(RuntimeError::PhysicsUnsupported))
    ));
    assert_same_time(&runtime, initial_time);
    assert_eq!(runtime.lifecycle_frame_count(), 0);

    runtime.physics_mode_write(1).unwrap();
    runtime.time_begin_frame().unwrap();
    runtime.lifecycle_begin_phase(1).unwrap();
    drain_timed_phase(&mut runtime);
    runtime.physics_mode_write(0).unwrap();

    let before_fixed = runtime.time_snapshot().unwrap();
    assert_eq!(
        runtime.time_begin_fixed(),
        Err(RuntimeError::PhysicsUnsupported)
    );
    assert_same_time(&runtime, before_fixed);
    assert_eq!(runtime.lifecycle_frame_count(), 1);

    runtime.physics_mode_write(1).unwrap();
    assert!(runtime.time_begin_fixed().unwrap());
    assert_eq!(runtime.lifecycle_frame_count(), 1);
}

#[test]
fn fixed_mode_allows_a_reserved_body_until_commit() {
    let mut runtime = Runtime::default();
    let owner = runtime.create_gameobject("reserved-fixed-owner").unwrap();
    runtime.physics_mode_write(0).unwrap();
    runtime.rigidbody_reserve(owner).unwrap();

    runtime.lifecycle_begin_frame().unwrap();
    assert_eq!(runtime.lifecycle_frame_count(), 1);
}
