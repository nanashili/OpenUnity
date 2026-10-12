//! Public Runtime owner tests for INFRA-PHYSICS-MOTION-001.
//!
//! These cases use the safe Runtime boundary. They intentionally avoid
//! `MotionWorld` internals and the forty-row managed reference fixture.

use engine_identifiers::ObjectId;
use engine_runtime::{Runtime, RuntimeError};

fn create_committed_body(runtime: &mut Runtime) -> (ObjectId, ObjectId, ObjectId) {
    let owner = runtime.create_gameobject("motion-owner").unwrap();
    let transform = runtime.gameobject_transform(owner).unwrap();
    let body = runtime.rigidbody_reserve(owner).unwrap();
    runtime.rigidbody_commit(body).unwrap();
    (owner, transform, body)
}

#[test]
fn fixed_mode_rejects_manual_step_without_mutating_the_owned_transform() {
    // Audit: protects the public mode admission invariant. A credible
    // regression would run a body during Fixed mode or publish a position
    // before returning the unsupported-mode error. ObjectStore tests cannot
    // reach Runtime mode or Transform publication.
    let mut runtime = Runtime::default();
    let (_, transform, _) = create_committed_body(&mut runtime);
    let before = runtime.transform_read(transform, 1).unwrap();

    assert_eq!(
        runtime.physics_simulate(0.25),
        Err(RuntimeError::PhysicsUnsupported)
    );
    assert_eq!(runtime.transform_read(transform, 1).unwrap(), before);
}

#[test]
fn dirty_transform_requires_sync_before_manual_step_and_then_publishes_position() {
    // Audit: protects Runtime's authored-position revision gate and explicit
    // SyncTransforms handoff. Existing backend tests do not own Transform
    // dirtiness, and ObjectStore tests do not contain a body or a mode.
    let mut runtime = Runtime::default();
    let (_, transform, body) = create_committed_body(&mut runtime);
    runtime.physics_mode_write(1).unwrap();
    runtime.set_physics_gravity([0.0, -4.0, 0.0]).unwrap();
    runtime
        .transform_write(transform, 1, &[3.0, 0.0, 0.0])
        .unwrap();

    assert_eq!(runtime.physics_simulate(0.25), Err(RuntimeError::Lifecycle));
    assert_eq!(runtime.physics_sync_transforms(), Ok(()));
    runtime.physics_simulate(0.25).unwrap();

    assert_eq!(
        runtime.transform_read(transform, 1).unwrap(),
        vec![3.0, -0.25, 0.0]
    );
    assert_eq!(
        runtime.rigidbody_state(body).unwrap().position(),
        [3.0, -0.25, 0.0]
    );
}

#[test]
fn attached_transform_rejects_parent_change_before_graph_mutation() {
    // Audit: protects the Runtime boundary that rejects unsupported hierarchy
    // edits for an attached body before mutating either Transform graph. The
    // ObjectStore owner tests do not model Transform parents or body policy.
    let mut runtime = Runtime::default();
    let (_, transform, _) = create_committed_body(&mut runtime);
    let parent_owner = runtime.create_gameobject("parent").unwrap();
    let parent = runtime.gameobject_transform(parent_owner).unwrap();

    assert_eq!(
        runtime.transform_set_parent(transform, Some(parent), false),
        Err(RuntimeError::PhysicsUnsupported)
    );
    assert_eq!(runtime.transform_parent(transform).unwrap(), None);
    assert_eq!(runtime.transform_child_count(parent).unwrap(), 0);
}

#[test]
fn committed_body_retirement_keeps_owner_then_owner_retirement_cleans_scene() {
    // Audit: protects the public retirement transaction across both targets.
    // Direct body retirement must preserve the owner and Transform, while
    // owner retirement must remove the replacement body and mandatory pair.
    let mut runtime = Runtime::default();
    let (owner, transform, body) = create_committed_body(&mut runtime);

    runtime.destroy_typed_object(body).unwrap();
    assert!(!runtime.is_alive(body));
    assert!(runtime.is_alive(owner));
    assert!(runtime.is_alive(transform));

    let replacement = runtime.rigidbody_reserve(owner).unwrap();
    runtime.rigidbody_commit(replacement).unwrap();
    runtime.destroy_typed_object(owner).unwrap();

    assert!(!runtime.is_alive(owner));
    assert!(!runtime.is_alive(transform));
    assert!(!runtime.is_alive(replacement));

    runtime.physics_mode_write(1).unwrap();
    runtime.physics_simulate(0.25).unwrap();
}
