//! Stage2 public Runtime acceptance cases for retirement and authored revision state.
//!
//! Approved contract: `docs/specifications/physics/INFRA-PHYSICS-MOTION-001-stage2-candidate.md`.

use engine_identifiers::ObjectId;
use engine_runtime::{Runtime, RuntimeError};

fn create_committed_body(runtime: &mut Runtime) -> (ObjectId, ObjectId, ObjectId) {
    let owner = runtime
        .create_gameobject("stage2-retirement-owner")
        .unwrap();
    let transform = runtime.gameobject_transform(owner).unwrap();
    let body = runtime.rigidbody_reserve(owner).unwrap();
    runtime.rigidbody_commit(body).unwrap();
    (owner, transform, body)
}

fn drain_phase(runtime: &mut Runtime) {
    while let Some(delivery) = runtime.lifecycle_next(1).unwrap() {
        runtime.lifecycle_ack(delivery.token, 0).unwrap();
    }
}

fn prepare_started_script(runtime: &mut Runtime, owner: ObjectId) -> ObjectId {
    let script = runtime.component_reserve(owner, 0).unwrap();
    runtime.component_commit(script).unwrap();
    runtime.physics_mode_write(1).unwrap();
    runtime.lifecycle_begin_frame().unwrap();
    runtime.lifecycle_begin_phase(1).unwrap();
    drain_phase(runtime);
    script
}

#[test]
fn finite_step_overflow_preserves_body_and_transform_state() {
    let mut runtime = Runtime::default();
    let (_, transform, body) = create_committed_body(&mut runtime);
    runtime.physics_mode_write(1).unwrap();
    runtime.rigidbody_set_use_gravity(body, false).unwrap();
    runtime
        .rigidbody_set_linear_velocity(body, [f32::MAX, 0.0, 0.0])
        .unwrap();

    let transform_before = runtime.transform_read(transform, 1).unwrap();
    let body_before = runtime.rigidbody_state(body).unwrap();

    assert_eq!(runtime.physics_simulate(2.0), Err(RuntimeError::Numeric));
    assert_eq!(
        runtime.transform_read(transform, 1).unwrap(),
        transform_before
    );
    assert_eq!(runtime.rigidbody_state(body).unwrap(), body_before);
}

#[test]
fn newer_body_position_write_acknowledges_dirty_revision_without_transform_writeback() {
    let mut runtime = Runtime::default();
    let (_, transform, body) = create_committed_body(&mut runtime);
    runtime
        .transform_write(transform, 1, &[3.0, 0.0, 0.0])
        .unwrap();
    let authored_transform = runtime.transform_read(transform, 1).unwrap();

    runtime
        .rigidbody_set_position(body, [9.0, 0.0, 0.0])
        .unwrap();

    assert_eq!(
        runtime.transform_read(transform, 1).unwrap(),
        authored_transform
    );
    assert_eq!(
        runtime.rigidbody_state(body).unwrap().position(),
        [9.0, 0.0, 0.0]
    );
    runtime.physics_sync_transforms().unwrap();
    assert_eq!(
        runtime.transform_read(transform, 1).unwrap(),
        authored_transform
    );
    assert_eq!(
        runtime.rigidbody_state(body).unwrap().position(),
        [9.0, 0.0, 0.0]
    );
}

#[test]
fn pending_body_rejects_writes_during_disable_and_destroy_then_retires_after_delivery() {
    let mut runtime = Runtime::default();
    let owner = runtime.create_gameobject("body-callback-owner").unwrap();
    let transform = runtime.gameobject_transform(owner).unwrap();
    let body = runtime.rigidbody_reserve(owner).unwrap();
    runtime.rigidbody_commit(body).unwrap();
    let script = prepare_started_script(&mut runtime, owner);

    runtime.lifecycle_begin_phase(2).unwrap();
    drain_phase(&mut runtime);
    runtime.lifecycle_begin_phase(3).unwrap();
    drain_phase(&mut runtime);
    runtime.lifecycle_begin_phase(4).unwrap();
    drain_phase(&mut runtime);
    runtime.object_destroy_deferred(owner).unwrap();
    runtime.lifecycle_begin_phase(5).unwrap();

    let disable = runtime.lifecycle_next(1).unwrap().unwrap();
    assert_eq!(disable.component, script);
    assert_eq!(disable.event, 6);
    assert_eq!(
        runtime.rigidbody_set_linear_velocity(body, [1.0, 0.0, 0.0]),
        Err(RuntimeError::Lifecycle)
    );
    assert!(runtime.rigidbody_state(body).is_ok());
    assert_eq!(runtime.physics_component_count(owner).unwrap(), 3);
    assert_eq!(runtime.physics_component_at(owner, 0).unwrap(), transform);
    assert_eq!(runtime.physics_component_at(owner, 1).unwrap(), body);
    assert_eq!(runtime.physics_component_at(owner, 2).unwrap(), script);
    runtime.lifecycle_ack(disable.token, 0).unwrap();

    let destroy = runtime.lifecycle_next(1).unwrap().unwrap();
    assert_eq!(destroy.component, script);
    assert_eq!(destroy.event, 7);
    assert_eq!(
        runtime.rigidbody_set_position(body, [2.0, 0.0, 0.0]),
        Err(RuntimeError::Lifecycle)
    );
    assert!(runtime.rigidbody_state(body).is_ok());
    assert_eq!(runtime.physics_component_at(owner, 1).unwrap(), body);
    runtime.lifecycle_ack(destroy.token, 0).unwrap();

    assert!(runtime.lifecycle_next(1).unwrap().is_none());
    assert_eq!(
        runtime.rigidbody_state(body),
        Err(RuntimeError::InvalidObject)
    );
    assert_eq!(
        runtime.physics_component_count(owner),
        Err(RuntimeError::InvalidObject)
    );
    assert!(!runtime.is_alive(owner));
    assert!(!runtime.is_alive(transform));
}
