//! Query-record ownership across runtime lifecycle transitions.

use engine_runtime::{Runtime, RuntimeError};
use engine_serialization::{GameObjectPlan, ScenePlan, TransformPlan};

fn empty_scene() -> ScenePlan {
    ScenePlan {
        schema_version: 1,
        scene_name: "query-lifecycle".to_owned(),
        roots: vec![2],
        game_objects: vec![GameObjectPlan {
            file_id: 1,
            name: "owner".to_owned(),
            active: true,
            components: vec![2],
        }],
        transforms: vec![TransformPlan {
            file_id: 2,
            owner: 1,
            parent: 0,
            children: Vec::new(),
            local_position: [0.0; 3],
            local_rotation: [0.0, 0.0, 0.0, 1.0],
            local_scale: [1.0; 3],
        }],
        scripts: Vec::new(),
    }
}

#[test]
fn subtree_retirement_removes_each_owner_record() {
    let mut runtime = Runtime::default();
    let parent = runtime.create_gameobject("parent").unwrap();
    let child = runtime.create_gameobject("child").unwrap();
    let child_transform = runtime.gameobject_transform(child).unwrap();
    let parent_transform = runtime.gameobject_transform(parent).unwrap();

    runtime
        .transform_set_parent(child_transform, Some(parent_transform), false)
        .unwrap();
    runtime
        .physics_aabb_add(parent, [0.0; 3], [1.0; 3])
        .unwrap();
    runtime.physics_aabb_add(child, [2.0; 3], [3.0; 3]).unwrap();

    runtime.destroy_typed_object(parent).unwrap();

    assert_eq!(
        runtime.physics_overlap_aabb([0.0; 3], [4.0; 3]),
        Ok(Vec::new())
    );
}

#[test]
fn component_retirement_preserves_unrelated_owner_record() {
    let mut runtime = Runtime::default();
    let owner = runtime.create_gameobject("owner").unwrap();
    let other = runtime.create_gameobject("other").unwrap();
    let script = runtime.component_reserve(owner, 0).unwrap();
    runtime.component_commit(script).unwrap();
    runtime.physics_aabb_add(owner, [0.0; 3], [1.0; 3]).unwrap();
    runtime.physics_aabb_add(other, [2.0; 3], [3.0; 3]).unwrap();

    runtime.object_destroy_deferred(script).unwrap();
    runtime.lifecycle_flush_destroy().unwrap();

    assert_eq!(
        runtime.physics_overlap_aabb([0.0; 3], [1.0; 3]),
        Ok(vec![owner])
    );
    assert!(runtime.is_alive(owner));
}

#[test]
fn pending_owner_remains_query_visible_until_finalization() {
    let mut runtime = Runtime::default();
    let owner = runtime.create_gameobject("owner").unwrap();
    runtime.physics_aabb_add(owner, [0.0; 3], [1.0; 3]).unwrap();

    runtime.object_destroy_deferred(owner).unwrap();

    assert_eq!(
        runtime.physics_overlap_aabb([0.0; 3], [1.0; 3]),
        Ok(vec![owner])
    );
}

#[test]
fn scene_release_removes_owned_query_records() {
    let mut runtime = Runtime::default();
    let handle = runtime.scene_stage(&empty_scene()).unwrap();
    runtime.scene_publish(handle).unwrap();
    let owner = runtime.scene_object(handle, 1).unwrap();
    runtime.physics_aabb_add(owner, [0.0; 3], [1.0; 3]).unwrap();

    runtime.scene_release(handle).unwrap();

    assert_eq!(
        runtime.physics_overlap_aabb([0.0; 3], [1.0; 3]),
        Ok(Vec::new())
    );
    assert_eq!(
        runtime.scene_object(handle, 1),
        Err(RuntimeError::InvalidScene)
    );
}

#[test]
fn scene_release_after_owner_destruction_ignores_stale_owner_and_preserves_unrelated_query() {
    let mut runtime = Runtime::default();
    let handle = runtime.scene_stage(&empty_scene()).unwrap();
    runtime.scene_publish(handle).unwrap();

    let scene_owner = runtime.scene_object(handle, 1).unwrap();
    let unrelated_owner = runtime.create_gameobject("unrelated").unwrap();
    runtime
        .physics_aabb_add(scene_owner, [0.0; 3], [1.0; 3])
        .unwrap();
    runtime
        .physics_aabb_add(unrelated_owner, [2.0; 3], [3.0; 3])
        .unwrap();

    runtime.scene_destroy(handle).unwrap();
    runtime.lifecycle_flush_destroy().unwrap();
    assert!(runtime.lifecycle_next(1).unwrap().is_none());

    assert!(!runtime.is_alive(scene_owner));
    runtime.scene_release(handle).unwrap();

    assert_eq!(
        runtime.physics_overlap_aabb([0.0; 3], [3.0; 3]),
        Ok(vec![unrelated_owner])
    );
}
