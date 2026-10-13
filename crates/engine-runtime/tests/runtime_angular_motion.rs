//! Runtime property and lifetime coverage for stored angular velocity.

use engine_identifiers::ObjectId;
use engine_runtime::{Runtime, RuntimeError};

fn create_committed_body(runtime: &mut Runtime) -> (ObjectId, ObjectId, ObjectId) {
    let owner = runtime.create_gameobject("angular-owner").unwrap();
    let transform = runtime.gameobject_transform(owner).unwrap();
    let body = runtime.rigidbody_reserve(owner).unwrap();
    runtime.rigidbody_commit(body).unwrap();
    (owner, transform, body)
}

#[test]
fn angular_values_survive_other_property_writes_and_transform_sync() {
    let mut runtime = Runtime::default();
    let (_, transform, body) = create_committed_body(&mut runtime);
    let angular_velocity = [0.5, -1.25, 2.0];

    runtime
        .rigidbody_set_angular_velocity(body, angular_velocity)
        .unwrap();
    runtime
        .rigidbody_set_linear_velocity(body, [3.0, 0.0, -2.0])
        .unwrap();
    runtime.rigidbody_set_mass(body, 4.0).unwrap();

    assert_eq!(
        runtime
            .rigidbody_state(body)
            .unwrap()
            .configuration()
            .angular_velocity(),
        angular_velocity
    );

    runtime
        .transform_write(transform, 1, &[5.0, 6.0, 7.0])
        .unwrap();
    runtime.physics_sync_transforms().unwrap();

    let synchronized = runtime.rigidbody_state(body).unwrap();
    assert_eq!(synchronized.position(), [5.0, 6.0, 7.0]);
    assert_eq!(
        synchronized.configuration().angular_velocity(),
        angular_velocity
    );
}

#[test]
fn unsupported_step_preserves_body_and_transform_then_retirement_keeps_owner_identity() {
    let mut runtime = Runtime::default();
    let (owner, transform, body) = create_committed_body(&mut runtime);
    runtime.physics_mode_write(1).unwrap();
    runtime.set_physics_gravity([0.0; 3]).unwrap();
    runtime
        .rigidbody_set_angular_velocity(body, [0.0, 2.0, 0.0])
        .unwrap();
    let previous_body = runtime.rigidbody_state(body).unwrap();
    let previous_transform = runtime.transform_read(transform, 1).unwrap();

    assert_eq!(
        runtime.physics_simulate(0.25),
        Err(RuntimeError::PhysicsUnsupported)
    );
    assert_eq!(runtime.rigidbody_state(body).unwrap(), previous_body);
    assert_eq!(
        runtime.transform_read(transform, 1).unwrap(),
        previous_transform
    );

    runtime.destroy_typed_object(body).unwrap();

    assert!(!runtime.is_alive(body));
    assert!(runtime.is_alive(owner));
    assert!(runtime.is_alive(transform));

    let replacement = runtime.rigidbody_reserve(owner).unwrap();
    runtime.rigidbody_commit(replacement).unwrap();
    assert_ne!(replacement, body);
    assert_eq!(
        runtime.rigidbody_state(replacement).unwrap().position(),
        [0.0; 3]
    );
}

#[test]
fn zero_reset_recovers_script_motion_after_angular_rejection() {
    let mut runtime = Runtime::default();
    let (_, transform, body) = create_committed_body(&mut runtime);
    runtime.physics_mode_write(1).unwrap();
    runtime.set_physics_gravity([0.0; 3]).unwrap();
    runtime
        .rigidbody_set_linear_velocity(body, [2.0, 0.0, 0.0])
        .unwrap();
    runtime
        .rigidbody_set_angular_velocity(body, [0.0, 1.0, 0.0])
        .unwrap();

    assert_eq!(
        runtime.physics_simulate(0.25),
        Err(RuntimeError::PhysicsUnsupported)
    );
    runtime
        .rigidbody_set_angular_velocity(body, [-0.0, 0.0, -0.0])
        .unwrap();
    runtime.physics_simulate(0.25).unwrap();

    assert_eq!(
        runtime.transform_read(transform, 1).unwrap(),
        vec![0.5, 0.0, 0.0]
    );
    let recovered = runtime.rigidbody_state(body).unwrap();
    assert_eq!(recovered.position(), [0.5, 0.0, 0.0]);
    assert_eq!(recovered.linear_velocity(), [2.0, 0.0, 0.0]);
    assert_eq!(
        recovered.configuration().angular_velocity(),
        [-0.0, 0.0, -0.0]
    );
}
