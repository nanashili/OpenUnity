//! Public FFI acceptance: selector, buffer, sentinel, and numeric precedence.

use openunity_engine::*;

fn committed_body(runtime: i64) -> i64 {
    let owner = unsafe { ou_gameobject_create(runtime, b"body".as_ptr(), 4) };
    let body = ou_rigidbody_reserve(runtime, owner);
    assert_eq!(ou_rigidbody_commit(runtime, body), 0);
    body
}

#[test]
fn failed_reads_preserve_sentinels_and_validation_precedes_runtime_lookup() {
    let runtime = ou_runtime_create();
    let body = committed_body(runtime);
    let mut scalar = 41.0_f32;
    let mut vector = [41.0_f32; 3];
    let mut flags = 41_i32;

    assert_eq!(
        unsafe { ou_rigidbody_scalar_read(runtime, body, 0, &mut scalar) },
        INVALID_KIND
    );
    assert_eq!(scalar, 41.0);
    assert_eq!(
        unsafe { ou_rigidbody_scalar_read(runtime, body, 1, std::ptr::null_mut()) },
        INVALID_BUFFER
    );
    assert_eq!(scalar, 41.0);
    assert_eq!(
        unsafe { ou_rigidbody_vector_read(runtime, body, 1, vector.as_mut_ptr(), 2) },
        INVALID_BUFFER
    );
    assert_eq!(vector, [41.0; 3]);
    assert_eq!(
        unsafe { ou_rigidbody_flag_read(runtime, body, 9, &mut flags) },
        INVALID_KIND
    );
    assert_eq!(flags, 41);

    let invalid_runtime = 9_999_i64;
    assert_eq!(
        ou_rigidbody_scalar_write(invalid_runtime, body, 0, 1.0),
        INVALID_KIND
    );
    assert_eq!(
        unsafe { ou_rigidbody_vector_write(invalid_runtime, body, 1, vector.as_ptr(), 2) },
        INVALID_BUFFER
    );
    assert_eq!(
        unsafe { ou_physics_gravity_write(invalid_runtime, vector.as_ptr(), 2) },
        INVALID_BUFFER
    );
    assert_eq!(ou_runtime_destroy(runtime), 0);
}

#[test]
fn nonfinite_writes_are_rejected_before_mutation() {
    let runtime = ou_runtime_create();
    let body = committed_body(runtime);
    let mut before = [0.0_f32; 3];
    let invalid = [f32::NAN, 0.0, 0.0];

    assert_eq!(
        unsafe { ou_rigidbody_vector_read(runtime, body, 1, before.as_mut_ptr(), 3) },
        0
    );
    assert_eq!(
        unsafe { ou_rigidbody_vector_write(runtime, body, 1, invalid.as_ptr(), 3) },
        INVALID_NUMERIC
    );
    assert_eq!(
        unsafe { ou_rigidbody_vector_read(runtime, body, 1, before.as_mut_ptr(), 3) },
        0
    );
    assert_eq!(before, [0.0; 3]);
    assert_eq!(
        unsafe { ou_physics_gravity_write(runtime, invalid.as_ptr(), 3) },
        INVALID_NUMERIC
    );
    assert_eq!(ou_runtime_destroy(runtime), 0);
}

#[test]
fn foreign_context_reads_reject_handles_and_preserve_output_sentinels() {
    let local = ou_runtime_create();
    let foreign = ou_runtime_create();
    let foreign_body = committed_body(foreign);
    let mut local_output = [17.0_f32; 3];
    let mut foreign_output = [17.0_f32; 3];

    assert_eq!(
        unsafe { ou_rigidbody_vector_read(local, foreign_body, 1, local_output.as_mut_ptr(), 3) },
        INVALID_OBJECT
    );
    assert_eq!(local_output, [17.0; 3]);
    assert_eq!(
        unsafe {
            ou_rigidbody_vector_read(foreign, foreign_body, 1, foreign_output.as_mut_ptr(), 3)
        },
        0
    );
    assert_eq!(foreign_output, [0.0; 3]);

    assert_eq!(ou_runtime_destroy(local), 0);
    assert_eq!(ou_runtime_destroy(foreign), 0);
}
