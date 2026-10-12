//! Public FFI acceptance: ABI identity, context isolation, and teardown.

use openunity_engine::*;

fn game_object(runtime: i64, name: &[u8]) -> i64 {
    unsafe { ou_gameobject_create(runtime, name.as_ptr(), name.len() as i32) }
}

#[test]
fn versions_are_storage_independent_and_contexts_are_isolated() {
    assert_eq!(ou_physics_motion_abi_version(), 0x0001_0000);
    let first = ou_runtime_create();
    let second = ou_runtime_create();
    assert!(first > 0 && second > 0 && first != second);

    let first_object = ou_object_create(first);
    let second_object = ou_object_create(second);
    assert_eq!(ou_object_is_alive(first, first_object), 1);
    assert_eq!(ou_object_is_alive(first, second_object), 0);
    assert_eq!(ou_object_is_alive(second, first_object), 0);

    assert_eq!(ou_runtime_destroy(first), 0);
    assert_eq!(ou_object_is_alive(first, first_object), INVALID_RUNTIME);
    assert_eq!(ou_runtime_destroy(second), 0);
}

#[test]
fn stale_and_wrong_kind_handles_fail_without_cross_context_mutation() {
    let runtime = ou_runtime_create();
    let owner = game_object(runtime, b"owner");
    let transform = ou_gameobject_transform(runtime, owner);
    assert!(owner > 0 && transform > 0);

    assert_eq!(
        ou_rigidbody_reserve(runtime, transform),
        INVALID_KIND as i64
    );
    assert_eq!(ou_physics_component_count(runtime, transform), INVALID_KIND);
    assert_eq!(ou_object_destroy(runtime, transform), INVALID_KIND);
    assert_eq!(ou_object_is_alive(runtime, transform), 1);
    assert_eq!(ou_object_is_alive(runtime, owner), 1);
    assert_eq!(ou_runtime_destroy(runtime), 0);
    assert_eq!(ou_rigidbody_reserve(runtime, owner), INVALID_RUNTIME as i64);
    assert_eq!(ou_runtime_destroy(runtime), INVALID_RUNTIME);
}
