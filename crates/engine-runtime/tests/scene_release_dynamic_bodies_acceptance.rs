//! Public acceptance cases for releasing scene-owned dynamic bodies.

use engine_identifiers::ObjectId;
use engine_runtime::{Runtime, RuntimeError};
use engine_serialization::{GameObjectPlan, ScenePlan, TransformPlan};

fn single_owner_scene() -> ScenePlan {
    ScenePlan {
        schema_version: 1,
        scene_name: String::from("public-dynamic-release"),
        roots: vec![2],
        game_objects: vec![GameObjectPlan {
            file_id: 1,
            name: String::from("SceneOwner"),
            active: true,
            components: vec![2],
        }],
        transforms: vec![TransformPlan {
            file_id: 2,
            owner: 1,
            parent: 0,
            children: Vec::new(),
            local_position: [0.0, 0.0, 0.0],
            local_rotation: [0.0, 0.0, 0.0, 1.0],
            local_scale: [1.0, 1.0, 1.0],
        }],
        scripts: Vec::new(),
    }
}

fn publish_scene(runtime: &mut Runtime) -> (ObjectId, ObjectId) {
    let scene = runtime
        .scene_stage(&single_owner_scene())
        .expect("stage scene");
    runtime.scene_publish(scene).expect("publish scene");

    let owner = runtime.scene_object(scene, 1).expect("resolve scene owner");
    (scene, owner)
}

fn assert_independent_owner_is_usable(runtime: &mut Runtime, owner: ObjectId) {
    assert!(runtime.is_alive(owner));
    assert_eq!(runtime.gameobject_active(owner, true), Ok(true));

    runtime
        .gameobject_set_active(owner, false)
        .expect("disable independent owner");
    assert_eq!(runtime.gameobject_active(owner, true), Ok(false));

    runtime
        .gameobject_set_active(owner, true)
        .expect("restore independent owner");
    assert_eq!(runtime.gameobject_active(owner, true), Ok(true));
}

#[test]
fn release_retires_committed_scene_body_and_preserves_independent_owner() {
    let mut runtime = Runtime::default();
    let independent_owner = runtime
        .create_gameobject("Independent")
        .expect("create independent owner");
    let (scene, scene_owner) = publish_scene(&mut runtime);

    let body = runtime
        .rigidbody_reserve(scene_owner)
        .expect("reserve scene body");
    runtime.rigidbody_commit(body).expect("commit scene body");
    assert_eq!(runtime.physics_component_count(scene_owner), Ok(2));
    assert_eq!(runtime.physics_component_at(scene_owner, 1), Ok(body));

    runtime.scene_release(scene).expect("release scene");

    assert!(!runtime.is_alive(scene_owner));
    assert!(!runtime.is_alive(body));
    assert_independent_owner_is_usable(&mut runtime, independent_owner);
}

#[test]
fn release_retires_reserved_scene_body_and_preserves_independent_owner() {
    let mut runtime = Runtime::default();
    let independent_owner = runtime
        .create_gameobject("Independent")
        .expect("create independent owner");
    let (scene, scene_owner) = publish_scene(&mut runtime);

    let body = runtime
        .rigidbody_reserve(scene_owner)
        .expect("reserve scene body");
    assert_eq!(runtime.physics_component_count(scene_owner), Ok(1));

    runtime.scene_release(scene).expect("release scene");

    assert!(!runtime.is_alive(scene_owner));
    assert!(!runtime.is_alive(body));
    assert_independent_owner_is_usable(&mut runtime, independent_owner);
}

#[test]
fn pending_scene_body_destruction_blocks_release_until_flushed() {
    let mut runtime = Runtime::default();
    let (scene, owner) = publish_scene(&mut runtime);
    let body = runtime.rigidbody_reserve(owner).expect("reserve body");
    runtime.rigidbody_commit(body).expect("publish body");
    runtime
        .object_destroy_deferred(body)
        .expect("request body destruction");

    assert_eq!(runtime.scene_release(scene), Err(RuntimeError::StagingBusy));
    assert_eq!(runtime.scene_object(scene, 1), Ok(owner));
    assert!(runtime.is_alive(body));

    runtime
        .lifecycle_flush_destroy()
        .expect("begin body destruction");
    assert!(
        runtime
            .lifecycle_next(1)
            .expect("flush body destruction")
            .is_none()
    );
    assert!(!runtime.is_alive(body));
    assert!(runtime.is_alive(owner));
    runtime.scene_release(scene).expect("release after flush");
    runtime
        .lifecycle_flush_destroy()
        .expect("begin empty flush");
    assert!(
        runtime
            .lifecycle_next(1)
            .expect("no stale destruction target")
            .is_none()
    );
}

#[test]
fn unrelated_pending_destruction_survives_scene_release() {
    let mut runtime = Runtime::default();
    let independent = runtime
        .create_gameobject("Independent")
        .expect("create owner");
    let (scene, owner) = publish_scene(&mut runtime);
    let body = runtime.rigidbody_reserve(owner).expect("reserve body");
    runtime.rigidbody_commit(body).expect("publish body");
    runtime
        .object_destroy_deferred(independent)
        .expect("request unrelated destruction");

    runtime
        .scene_release(scene)
        .expect("release unrelated scene");
    assert!(runtime.is_alive(independent));
    assert!(!runtime.is_alive(body));

    runtime
        .lifecycle_flush_destroy()
        .expect("begin retained destruction");
    assert!(
        runtime
            .lifecycle_next(1)
            .expect("flush retained unrelated request")
            .is_none()
    );
    assert!(!runtime.is_alive(independent));
    runtime
        .lifecycle_flush_destroy()
        .expect("begin empty flush");
    assert!(
        runtime
            .lifecycle_next(1)
            .expect("no stale destruction target")
            .is_none()
    );
}
