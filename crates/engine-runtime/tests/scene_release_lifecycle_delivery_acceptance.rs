//! Public acceptance cases for scene release around lifecycle delivery.

use engine_identifiers::ObjectId;
use engine_runtime::{Runtime, RuntimeError};
use engine_serialization::{GameObjectPlan, ScenePlan, TransformPlan};

fn single_owner_scene() -> ScenePlan {
    ScenePlan {
        schema_version: 1,
        scene_name: String::from("public-lifecycle-release"),
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

fn publish_scene_with_committed_body(runtime: &mut Runtime) -> (ObjectId, ObjectId) {
    let scene = runtime
        .scene_stage(&single_owner_scene())
        .expect("stage scene");
    runtime.scene_publish(scene).expect("publish scene");
    let owner = runtime.scene_object(scene, 1).expect("resolve scene owner");
    let body = runtime
        .rigidbody_reserve(owner)
        .expect("reserve scene body");
    runtime.rigidbody_commit(body).expect("commit scene body");
    (scene, body)
}

fn begin_unfinished_delivery(runtime: &mut Runtime) -> i64 {
    runtime
        .physics_mode_write(1)
        .expect("select manual simulation");
    let owner = runtime
        .create_gameobject("CallbackOwner")
        .expect("create callback owner");
    let script = runtime
        .component_reserve(owner, 0)
        .expect("reserve callback script");
    runtime
        .component_commit(script)
        .expect("commit callback script");

    runtime.lifecycle_begin_frame().expect("begin frame");
    runtime.lifecycle_begin_phase(1).expect("begin start phase");
    runtime
        .lifecycle_next(1)
        .expect("pull callback")
        .expect("callback delivery")
        .token
}

#[test]
fn scene_release_rejects_in_flight_delivery_before_body_retirement() {
    let mut runtime = Runtime::default();
    let (scene, body) = publish_scene_with_committed_body(&mut runtime);
    let token = begin_unfinished_delivery(&mut runtime);

    assert_eq!(runtime.scene_release(scene), Err(RuntimeError::StagingBusy));
    assert!(runtime.is_alive(body));

    runtime
        .lifecycle_ack(token, 0)
        .expect("acknowledge callback");
    runtime
        .scene_release(scene)
        .expect("release scene after ack");
    assert!(!runtime.is_alive(body));
}

#[test]
fn scene_release_cleans_body_after_fault_ack_while_dispatch_stays_faulted() {
    let mut runtime = Runtime::default();
    let (scene, body) = publish_scene_with_committed_body(&mut runtime);
    let token = begin_unfinished_delivery(&mut runtime);

    runtime
        .lifecycle_ack(token, -6)
        .expect("record callback fault");
    assert_eq!(
        runtime.lifecycle_begin_phase(2),
        Err(RuntimeError::DispatchFaulted)
    );

    runtime
        .scene_release(scene)
        .expect("release scene after fault acknowledgement");
    assert!(!runtime.is_alive(body));
}
