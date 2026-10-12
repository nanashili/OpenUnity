//! Public same-frame retry coverage for fixed-mode lifecycle admission.

use engine_identifiers::ObjectId;
use engine_runtime::{Runtime, RuntimeError, RuntimeInputError};

fn create_committed_body(runtime: &mut Runtime) -> (ObjectId, ObjectId) {
    let owner = runtime
        .create_gameobject("fixed-admission-retry-owner")
        .unwrap();
    let body = runtime.rigidbody_reserve(owner).unwrap();
    runtime.rigidbody_commit(body).unwrap();
    (owner, body)
}

fn assert_fixed_rejection<T>(result: Result<T, RuntimeInputError>) {
    assert!(matches!(
        result,
        Err(RuntimeInputError::Runtime(RuntimeError::PhysicsUnsupported))
    ));
}

fn drain_phase(runtime: &mut Runtime) -> Vec<i32> {
    let mut events = Vec::new();

    while let Some(delivery) = runtime.lifecycle_next(1).unwrap() {
        events.push(delivery.event);
        runtime.lifecycle_ack(delivery.token, 0).unwrap();
    }

    events
}

fn drain_timed_phase(runtime: &mut Runtime) {
    while let Some(delivery) = runtime.time_next(1).unwrap() {
        runtime.lifecycle_ack(delivery.token, 0).unwrap();
    }
}

#[test]
fn ordinary_outer_frame_retries_same_frame_after_mode_change() {
    let mut runtime = Runtime::default();
    create_committed_body(&mut runtime);
    runtime.physics_mode_write(0).unwrap();

    assert_fixed_rejection(runtime.lifecycle_begin_frame());
    assert_eq!(runtime.lifecycle_frame_count(), 0);

    runtime.physics_mode_write(1).unwrap();
    runtime.lifecycle_begin_frame().unwrap();
    assert_eq!(runtime.lifecycle_frame_count(), 1);
}

#[test]
fn timed_outer_frame_retries_same_frame_after_mode_change() {
    let mut runtime = Runtime::default();
    create_committed_body(&mut runtime);
    runtime.physics_mode_write(0).unwrap();
    runtime.time_configure(1, 0.02).unwrap();
    let before = runtime.time_snapshot().unwrap();

    assert_fixed_rejection(runtime.time_begin_frame());

    let rejected = runtime.time_snapshot().unwrap();
    assert_eq!(rejected.values, before.values);
    assert_eq!(rejected.frame, before.frame);
    assert_eq!(runtime.lifecycle_frame_count(), 0);

    runtime.physics_mode_write(1).unwrap();
    runtime.time_begin_frame().unwrap();
    assert_eq!(runtime.lifecycle_frame_count(), 1);
}

#[test]
fn due_fixed_step_retries_after_mode_change_without_clock_loss() {
    let mut runtime = Runtime::default();
    runtime.time_configure(1, 0.02).unwrap();
    runtime.time_set(3, 0.02).unwrap();
    runtime.time_begin_frame().unwrap();
    runtime.lifecycle_begin_phase(1).unwrap();
    drain_timed_phase(&mut runtime);

    create_committed_body(&mut runtime);
    runtime.physics_mode_write(0).unwrap();
    let before = runtime.time_snapshot().unwrap();
    assert_eq!(
        runtime.time_begin_fixed(),
        Err(RuntimeError::PhysicsUnsupported)
    );

    let rejected = runtime.time_snapshot().unwrap();
    assert_eq!(rejected.values, before.values);
    assert_eq!(rejected.frame, before.frame);

    runtime.physics_mode_write(1).unwrap();
    assert!(runtime.time_begin_fixed().unwrap());
    assert_eq!(runtime.lifecycle_frame_count(), 1);
}

#[test]
fn zero_due_fixed_slot_retries_after_mode_change_without_clock_transition() {
    let mut runtime = Runtime::default();
    runtime.time_configure(1, 0.02).unwrap();
    runtime.time_set(1, 0.0).unwrap();
    runtime.time_begin_frame().unwrap();
    runtime.lifecycle_begin_phase(1).unwrap();
    drain_timed_phase(&mut runtime);

    create_committed_body(&mut runtime);
    runtime.physics_mode_write(0).unwrap();
    let before = runtime.time_snapshot().unwrap();
    assert_eq!(
        runtime.time_begin_fixed(),
        Err(RuntimeError::PhysicsUnsupported)
    );

    let rejected = runtime.time_snapshot().unwrap();
    assert_eq!(rejected.values, before.values);
    assert_eq!(rejected.frame, before.frame);

    runtime.physics_mode_write(1).unwrap();
    assert!(!runtime.time_begin_fixed().unwrap());
    assert_eq!(runtime.lifecycle_frame_count(), 1);
}

#[test]
fn rejected_phase_two_retry_does_not_redeliver_start() {
    let mut runtime = Runtime::default();
    let owner = runtime.create_gameobject("start-once-owner").unwrap();
    let script = runtime.component_reserve(owner, 0).unwrap();
    runtime.component_commit(script).unwrap();
    runtime.lifecycle_begin_frame().unwrap();
    runtime.lifecycle_begin_phase(1).unwrap();

    let initial_events = drain_phase(&mut runtime);
    assert_eq!(initial_events, vec![1, 2, 3]);

    let body = runtime.rigidbody_reserve(owner).unwrap();
    runtime.rigidbody_commit(body).unwrap();
    runtime.physics_mode_write(0).unwrap();
    assert_eq!(
        runtime.lifecycle_begin_phase(2),
        Err(RuntimeError::PhysicsUnsupported)
    );
    assert_eq!(runtime.lifecycle_frame_count(), 1);

    runtime.physics_mode_write(1).unwrap();
    runtime.lifecycle_begin_phase(2).unwrap();
    let retried_events = drain_phase(&mut runtime);

    assert_eq!(retried_events, vec![4]);
    assert_eq!(runtime.lifecycle_frame_count(), 1);
}
