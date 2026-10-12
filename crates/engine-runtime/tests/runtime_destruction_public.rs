//! Public Runtime acceptance cases for Rigidbody retirement and destruction.
//!
//! These tests use only the public `Runtime` surface and are intentionally
//! separate from the existing motion and fixed-admission acceptance files.

use engine_identifiers::ObjectId;
use engine_runtime::{Runtime, RuntimeError};

fn create_committed_body(runtime: &mut Runtime) -> (ObjectId, ObjectId, ObjectId) {
    let owner = runtime.create_gameobject("retirement-owner").unwrap();
    let transform = runtime.gameobject_transform(owner).unwrap();
    let body = runtime.rigidbody_reserve(owner).unwrap();
    runtime.rigidbody_commit(body).unwrap();
    (owner, transform, body)
}

fn drain_phase(runtime: &mut Runtime) -> Vec<(ObjectId, i32)> {
    let mut events = Vec::new();

    while let Some(delivery) = runtime.lifecycle_next(1).unwrap() {
        events.push((delivery.component, delivery.event));
        runtime.lifecycle_ack(delivery.token, 0).unwrap();
    }

    events
}

#[test]
fn deferred_owner_retirement_removes_committed_body_and_mandatory_transform() {
    let mut runtime = Runtime::default();
    let (owner, transform, body) = create_committed_body(&mut runtime);

    runtime.object_destroy_deferred(owner).unwrap();
    runtime.lifecycle_flush_destroy().unwrap();
    assert!(runtime.lifecycle_next(1).unwrap().is_none());

    assert!(!runtime.is_alive(owner));
    assert!(!runtime.is_alive(transform));
    assert!(!runtime.is_alive(body));
    assert_eq!(
        runtime.rigidbody_state(body),
        Err(RuntimeError::InvalidObject)
    );
}

#[test]
fn owner_retirement_aborts_reserved_body_without_leaking_identity() {
    let mut runtime = Runtime::default();
    let owner = runtime
        .create_gameobject("reserved-retirement-owner")
        .unwrap();
    let transform = runtime.gameobject_transform(owner).unwrap();
    let body = runtime.rigidbody_reserve(owner).unwrap();

    runtime.destroy_typed_object(owner).unwrap();

    assert!(!runtime.is_alive(owner));
    assert!(!runtime.is_alive(transform));
    assert!(!runtime.is_alive(body));
    assert_eq!(
        runtime.rigidbody_abort(body),
        Err(RuntimeError::InvalidObject)
    );
}

#[test]
fn public_admission_cannot_construct_a_rigidbody_subtree() {
    let mut runtime = Runtime::default();
    let parent = runtime.create_gameobject("subtree-parent").unwrap();
    let child = runtime.create_gameobject("subtree-child").unwrap();
    let parent_transform = runtime.gameobject_transform(parent).unwrap();
    let child_transform = runtime.gameobject_transform(child).unwrap();
    runtime
        .transform_set_parent(child_transform, Some(parent_transform), false)
        .unwrap();

    assert_eq!(
        runtime.rigidbody_reserve(parent),
        Err(RuntimeError::PhysicsUnsupported)
    );
    assert_eq!(
        runtime.rigidbody_reserve(child),
        Err(RuntimeError::PhysicsUnsupported)
    );
    assert!(runtime.is_alive(parent));
    assert!(runtime.is_alive(child));
}

#[test]
fn overlapping_deferred_owner_requests_deliver_one_destroy_callback_per_script() {
    let mut runtime = Runtime::default();
    let parent = runtime.create_gameobject("callback-parent").unwrap();
    let child = runtime.create_gameobject("callback-child").unwrap();
    let parent_transform = runtime.gameobject_transform(parent).unwrap();
    let child_transform = runtime.gameobject_transform(child).unwrap();
    runtime
        .transform_set_parent(child_transform, Some(parent_transform), false)
        .unwrap();

    let parent_script = runtime.component_reserve(parent, 0).unwrap();
    runtime.component_commit(parent_script).unwrap();
    let child_script = runtime.component_reserve(child, 1).unwrap();
    runtime.component_commit(child_script).unwrap();

    runtime.lifecycle_begin_frame().unwrap();
    runtime.lifecycle_begin_phase(1).unwrap();
    let initial = drain_phase(&mut runtime);
    assert!(
        initial
            .iter()
            .any(|(id, event)| *id == parent_script && *event == 1)
    );
    assert!(
        initial
            .iter()
            .any(|(id, event)| *id == child_script && *event == 1)
    );

    runtime.lifecycle_begin_phase(2).unwrap();
    let delivery = runtime.lifecycle_next(1).unwrap().unwrap();
    assert_eq!(delivery.event, 4);
    runtime.object_destroy_deferred(parent).unwrap();
    runtime.lifecycle_ack(delivery.token, 0).unwrap();
    runtime.object_destroy_deferred(child).unwrap();
    drain_phase(&mut runtime);

    runtime.lifecycle_begin_phase(3).unwrap();
    drain_phase(&mut runtime);
    runtime.lifecycle_begin_phase(4).unwrap();
    drain_phase(&mut runtime);
    runtime.lifecycle_begin_phase(5).unwrap();
    let destruction = drain_phase(&mut runtime);

    let callbacks: Vec<_> = destruction
        .iter()
        .filter(|(_, event)| *event == 7)
        .map(|(id, _)| *id)
        .collect();
    assert_eq!(callbacks, vec![parent_script, child_script]);
    assert!(!runtime.is_alive(parent));
    assert!(!runtime.is_alive(child));
    assert!(!runtime.is_alive(parent_script));
    assert!(!runtime.is_alive(child_script));
}

#[test]
fn failed_property_and_step_preserve_body_transform_and_dirty_state() {
    let mut runtime = Runtime::default();
    let (_, transform, body) = create_committed_body(&mut runtime);
    runtime.physics_mode_write(1).unwrap();
    let original_transform = runtime.transform_read(transform, 1).unwrap();
    let original_body = runtime.rigidbody_state(body).unwrap().position();

    assert_eq!(
        runtime.rigidbody_set_mass(body, 0.0),
        Err(RuntimeError::Numeric)
    );
    assert_eq!(
        runtime.transform_read(transform, 1).unwrap(),
        original_transform
    );
    assert_eq!(
        runtime.rigidbody_state(body).unwrap().position(),
        original_body
    );

    runtime
        .transform_write(transform, 1, &[7.0, 2.0, 1.0])
        .unwrap();
    let dirty_transform = runtime.transform_read(transform, 1).unwrap();
    let dirty_body = runtime.rigidbody_state(body).unwrap().position();
    assert_eq!(
        runtime.physics_simulate(f32::NAN),
        Err(RuntimeError::Numeric)
    );
    assert_eq!(
        runtime.transform_read(transform, 1).unwrap(),
        dirty_transform
    );
    assert_eq!(
        runtime.rigidbody_state(body).unwrap().position(),
        dirty_body
    );
    assert_eq!(runtime.physics_simulate(0.25), Err(RuntimeError::Lifecycle));
    assert_eq!(
        runtime.transform_read(transform, 1).unwrap(),
        dirty_transform
    );
    assert_eq!(
        runtime.rigidbody_state(body).unwrap().position(),
        dirty_body
    );
}
