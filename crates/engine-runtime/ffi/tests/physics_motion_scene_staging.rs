//! Public FFI acceptance: copied scene staging and staging-state precedence.

use openunity_engine::*;

const PLAN: &[u8] = br#"{
    "schema_version": 1,
    "scene_name": "ffi",
    "roots": [10],
    "game_objects": [{
        "file_id": 1,
        "name": "Root",
        "active": true,
        "components": [10]
    }],
    "transforms": [{
        "file_id": 10,
        "owner": 1,
        "parent": 0,
        "children": [],
        "local_position": [0.0, 0.0, 0.0],
        "local_rotation": [0.0, 0.0, 0.0, 1.0],
        "local_scale": [1.0, 1.0, 1.0]
    }],
    "scripts": []
}"#;

#[test]
fn staging_copies_plan_and_blocks_non_scene_calls_until_consumed() {
    let runtime = ou_runtime_create();
    let mut bytes = PLAN.to_vec();
    let scene = unsafe { ou_scene_stage(runtime, bytes.as_ptr(), bytes.len() as i32) };
    bytes.fill(0);
    drop(bytes);
    assert!(scene > 0);
    assert!(ou_scene_object(runtime, scene, 1) > 0);
    assert_eq!(ou_object_create(runtime), STAGING_BUSY as i64);
    assert_eq!(ou_scene_publish(runtime, scene), 0);
    assert!(ou_scene_object(runtime, scene, 1) > 0);
    assert_eq!(ou_scene_release(runtime, scene), 0);
    assert_eq!(ou_scene_object(runtime, scene, 1), INVALID_SCENE as i64);
    assert_eq!(ou_runtime_destroy(runtime), 0);
}

#[test]
fn malformed_or_empty_plans_fail_before_runtime_lookup() {
    let malformed = b"{}";
    assert_eq!(
        unsafe { ou_scene_stage(9_999, malformed.as_ptr(), malformed.len() as i32) },
        INVALID_SCENE_PLAN as i64
    );
    assert_eq!(
        unsafe { ou_scene_stage(9_999, std::ptr::null(), 0) },
        INVALID_BUFFER as i64
    );
}
