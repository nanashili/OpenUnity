//! Public FFI acceptance: Rigidbody reservation, publication, and catalog order.

use openunity_engine::*;

fn create_owner(runtime: i64) -> (i64, i64) {
    let owner = unsafe { ou_gameobject_create(runtime, b"owner".as_ptr(), 5) };
    let transform = ou_gameobject_transform(runtime, owner);
    (owner, transform)
}

#[test]
fn reservation_commit_and_abort_preserve_catalog_invariants() {
    let runtime = ou_runtime_create();
    let (owner, _) = create_owner(runtime);
    assert_eq!(ou_physics_component_count(runtime, owner), 1);

    let body = ou_rigidbody_reserve(runtime, owner);
    assert!(body > 0);
    assert_eq!(ou_physics_component_count(runtime, owner), 1);
    assert_eq!(ou_rigidbody_commit(runtime, body), 0);
    assert_eq!(ou_physics_component_count(runtime, owner), 2);
    assert_eq!(ou_physics_component_at(runtime, owner, 1), body);
    assert_eq!(ou_rigidbody_commit(runtime, body), INVALID_LIFECYCLE);
    assert_eq!(
        ou_rigidbody_reserve(runtime, owner),
        PHYSICS_UNSUPPORTED as i64
    );

    assert_eq!(ou_object_destroy(runtime, body), 0);
    let second_owner = create_owner(runtime).0;
    let pending = ou_rigidbody_reserve(runtime, second_owner);
    assert!(pending > 0);
    assert_eq!(ou_rigidbody_abort(runtime, pending), 0);
    assert_eq!(ou_physics_component_count(runtime, second_owner), 1);
    assert_eq!(ou_rigidbody_abort(runtime, pending), INVALID_OBJECT);

    assert_eq!(ou_runtime_destroy(runtime), 0);
}

#[test]
fn catalog_and_component_owner_queries_reject_stale_and_invalid_indices() {
    let runtime = ou_runtime_create();
    let owner = create_owner(runtime).0;
    let body = ou_rigidbody_reserve(runtime, owner);
    assert_eq!(ou_rigidbody_commit(runtime, body), 0);
    assert_eq!(ou_component_gameobject(runtime, body), owner);
    assert_eq!(
        ou_physics_component_at(runtime, owner, -1),
        INVALID_KIND as i64
    );
    assert_eq!(
        ou_physics_component_at(runtime, owner, 9),
        INVALID_KIND as i64
    );

    assert_eq!(ou_object_destroy(runtime, body), 0);
    assert_eq!(
        ou_component_gameobject(runtime, body),
        INVALID_OBJECT as i64
    );
    assert_eq!(ou_physics_component_count(runtime, owner), 1);
    assert_eq!(ou_runtime_destroy(runtime), 0);
}
