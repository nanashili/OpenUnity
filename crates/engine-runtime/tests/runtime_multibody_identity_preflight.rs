//! Public identity-allocation acceptance for multi-body admission failures.
//!
//! This target deliberately has one test. It runs both process-global ID
//! probes in one controlled sequence without peer tests sharing its allocator.

use engine_runtime::{Runtime, RuntimeError};

#[test]
fn admission_rejections_leave_the_next_process_identity_unchanged() {
    assert_reserved_duplicate_rejection_preserves_the_next_identity();
    assert_full_world_rejection_preserves_the_next_identity();
}

fn assert_reserved_duplicate_rejection_preserves_the_next_identity() {
    let mut runtime = Runtime::default();
    let owner = runtime.create_gameobject("reserved-first-owner").unwrap();
    let transform = runtime.gameobject_transform(owner).unwrap();
    let original = runtime.rigidbody_reserve(owner).unwrap();

    assert_eq!(runtime.physics_component_count(owner), Ok(1));
    assert_eq!(runtime.physics_component_at(owner, 0), Ok(transform));
    assert!(runtime.is_alive(original));

    let marker = runtime.create_object().unwrap();
    assert_eq!(
        runtime.rigidbody_reserve(owner),
        Err(RuntimeError::PhysicsUnsupported)
    );
    assert_eq!(runtime.physics_component_count(owner), Ok(1));
    assert_eq!(runtime.physics_component_at(owner, 0), Ok(transform));

    let identity_probe = runtime.create_object().unwrap();
    assert_eq!(
        identity_probe.to_raw(),
        marker.to_raw() + 1,
        "duplicate-owner rejection must not allocate a Rigidbody identity"
    );

    runtime.rigidbody_commit(original).unwrap();
    assert_eq!(runtime.physics_component_count(owner), Ok(2));
    assert_eq!(runtime.physics_component_at(owner, 1), Ok(original));
}

fn assert_full_world_rejection_preserves_the_next_identity() {
    let mut runtime = Runtime::default();
    let mut reservations = Vec::with_capacity(128);

    for index in 0..256 {
        let owner = runtime
            .create_gameobject(&format!("capacity-owner-{index}"))
            .unwrap();
        let body = runtime.rigidbody_reserve(owner).unwrap();

        if index % 2 == 0 {
            runtime.rigidbody_commit(body).unwrap();
        } else {
            reservations.push(body);
        }
    }

    let blocked_owner = runtime.create_gameobject("capacity-retry-owner").unwrap();
    let blocked_transform = runtime.gameobject_transform(blocked_owner).unwrap();
    assert_eq!(
        runtime.rigidbody_reserve(blocked_owner),
        Err(RuntimeError::PhysicsUnsupported)
    );
    assert_eq!(runtime.physics_component_count(blocked_owner), Ok(1));
    assert_eq!(
        runtime.physics_component_at(blocked_owner, 0),
        Ok(blocked_transform)
    );

    let identity_probe = runtime.create_object().unwrap();
    assert_eq!(
        identity_probe.to_raw(),
        blocked_transform.to_raw() + 1,
        "capacity rejection must not allocate a Rigidbody identity"
    );

    runtime.rigidbody_abort(reservations[0]).unwrap();
    let retry = runtime.rigidbody_reserve(blocked_owner).unwrap();
    runtime.rigidbody_commit(retry).unwrap();
    assert_eq!(runtime.physics_component_count(blocked_owner), Ok(2));
    assert_eq!(runtime.physics_component_at(blocked_owner, 1), Ok(retry));
}
