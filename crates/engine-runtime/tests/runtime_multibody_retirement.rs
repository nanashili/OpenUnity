//! Public Runtime acceptance for multi-body scene-retirement transactions.

use engine_identifiers::ObjectId;
use engine_runtime::{Runtime, RuntimeError};
use engine_serialization::{GameObjectPlan, ScenePlan, TransformPlan};

fn create_body(runtime: &mut Runtime, name: &str) -> (ObjectId, ObjectId, ObjectId) {
    let owner = runtime.create_gameobject(name).unwrap();
    let transform = runtime.gameobject_transform(owner).unwrap();
    let body = runtime.rigidbody_reserve(owner).unwrap();
    runtime.rigidbody_commit(body).unwrap();
    (owner, transform, body)
}

fn three_owner_scene() -> ScenePlan {
    ScenePlan {
        schema_version: 1,
        scene_name: String::from("multi-body-retirement"),
        roots: vec![2, 4, 6],
        game_objects: vec![
            GameObjectPlan {
                file_id: 1,
                name: String::from("CommittedPendingOwner"),
                active: true,
                components: vec![2],
            },
            GameObjectPlan {
                file_id: 3,
                name: String::from("CommittedOwner"),
                active: true,
                components: vec![4],
            },
            GameObjectPlan {
                file_id: 5,
                name: String::from("ReservedOwner"),
                active: true,
                components: vec![6],
            },
        ],
        transforms: vec![
            TransformPlan {
                file_id: 2,
                owner: 1,
                parent: 0,
                children: Vec::new(),
                local_position: [0.0; 3],
                local_rotation: [0.0, 0.0, 0.0, 1.0],
                local_scale: [1.0; 3],
            },
            TransformPlan {
                file_id: 4,
                owner: 3,
                parent: 0,
                children: Vec::new(),
                local_position: [1.0, 0.0, 0.0],
                local_rotation: [0.0, 0.0, 0.0, 1.0],
                local_scale: [1.0; 3],
            },
            TransformPlan {
                file_id: 6,
                owner: 5,
                parent: 0,
                children: Vec::new(),
                local_position: [2.0, 0.0, 0.0],
                local_rotation: [0.0, 0.0, 0.0, 1.0],
                local_scale: [1.0; 3],
            },
        ],
        scripts: Vec::new(),
    }
}

#[test]
fn scene_retirement_failure_preserves_batch_then_retires_committed_and_reserved_bodies() {
    let mut runtime = Runtime::default();
    let (survivor_owner, survivor_transform, survivor_body) =
        create_body(&mut runtime, "unrelated-retirement-survivor");
    runtime
        .rigidbody_set_use_gravity(survivor_body, false)
        .unwrap();
    let survivor_state = runtime.rigidbody_state(survivor_body).unwrap();
    let survivor_pose = runtime.transform_read(survivor_transform, 1).unwrap();

    let scene = runtime.scene_stage(&three_owner_scene()).unwrap();
    runtime.scene_publish(scene).unwrap();
    let pending_owner = runtime.scene_object(scene, 1).unwrap();
    let committed_owner = runtime.scene_object(scene, 3).unwrap();
    let reserved_owner = runtime.scene_object(scene, 5).unwrap();
    let reserved_transform = runtime.gameobject_transform(reserved_owner).unwrap();

    let pending_body = runtime.rigidbody_reserve(pending_owner).unwrap();
    runtime.rigidbody_commit(pending_body).unwrap();
    let committed_body = runtime.rigidbody_reserve(committed_owner).unwrap();
    runtime.rigidbody_commit(committed_body).unwrap();
    let reserved_body = runtime.rigidbody_reserve(reserved_owner).unwrap();
    runtime.object_destroy_deferred(pending_body).unwrap();

    let committed_state = runtime.rigidbody_state(committed_body).unwrap();
    assert_eq!(runtime.scene_release(scene), Err(RuntimeError::StagingBusy));

    assert!(runtime.is_alive(pending_owner));
    assert!(runtime.is_alive(committed_owner));
    assert!(runtime.is_alive(reserved_owner));
    assert!(runtime.is_alive(pending_body));
    assert!(runtime.is_alive(committed_body));
    assert!(runtime.is_alive(reserved_body));
    assert_eq!(runtime.rigidbody_state(committed_body), Ok(committed_state));
    assert_eq!(runtime.physics_component_count(committed_owner), Ok(2));
    assert_eq!(
        runtime.physics_component_at(committed_owner, 1),
        Ok(committed_body)
    );
    assert_eq!(runtime.physics_component_count(reserved_owner), Ok(1));
    assert_eq!(
        runtime.physics_component_at(reserved_owner, 0),
        Ok(reserved_transform)
    );
    assert_eq!(runtime.rigidbody_state(survivor_body), Ok(survivor_state));
    assert_eq!(
        runtime
            .transform_read(survivor_transform, 1)
            .unwrap()
            .as_slice(),
        survivor_pose.as_slice()
    );

    runtime.lifecycle_flush_destroy().unwrap();
    assert!(runtime.lifecycle_next(1).unwrap().is_none());
    assert!(!runtime.is_alive(pending_body));
    assert!(runtime.is_alive(committed_owner));
    assert!(runtime.is_alive(committed_body));
    assert!(runtime.is_alive(reserved_owner));
    assert!(runtime.is_alive(reserved_body));

    runtime.scene_release(scene).unwrap();

    assert!(!runtime.is_alive(pending_owner));
    assert!(!runtime.is_alive(committed_owner));
    assert!(!runtime.is_alive(reserved_owner));
    assert!(!runtime.is_alive(committed_body));
    assert!(!runtime.is_alive(reserved_body));
    assert_eq!(
        runtime.rigidbody_state(reserved_body),
        Err(RuntimeError::InvalidObject)
    );
    assert!(runtime.is_alive(survivor_owner));
    assert_eq!(runtime.rigidbody_state(survivor_body), Ok(survivor_state));
    assert_eq!(
        runtime
            .transform_read(survivor_transform, 1)
            .unwrap()
            .as_slice(),
        survivor_pose.as_slice()
    );
}

#[test]
fn overlapping_owner_subtree_and_direct_body_requests_retire_each_body_once() {
    let mut runtime = Runtime::default();
    let (owner, _, body) = create_body(&mut runtime, "overlapping-owner-subtree");
    let (survivor_owner, survivor_transform, survivor_body) =
        create_body(&mut runtime, "overlapping-owner-survivor");
    runtime
        .rigidbody_set_use_gravity(survivor_body, false)
        .unwrap();
    let survivor_state = runtime.rigidbody_state(survivor_body).unwrap();
    let survivor_pose = runtime.transform_read(survivor_transform, 1).unwrap();

    // Owner destruction includes its subtree; the explicit body request
    // overlaps that retirement set and must not retire the same slot twice.
    runtime.object_destroy_deferred(owner).unwrap();
    runtime.object_destroy_deferred(body).unwrap();
    runtime.lifecycle_flush_destroy().unwrap();
    assert!(runtime.lifecycle_next(1).unwrap().is_none());

    assert!(!runtime.is_alive(owner));
    assert!(!runtime.is_alive(body));
    assert!(runtime.is_alive(survivor_owner));
    assert_eq!(runtime.rigidbody_state(survivor_body), Ok(survivor_state));
    assert_eq!(
        runtime.transform_read(survivor_transform, 1),
        Ok(survivor_pose)
    );
}
