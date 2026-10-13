//! Public Runtime admission cases for the PHYSICS-POSE-001 root-owner profile.

use engine_identifiers::ObjectId;
use engine_runtime::{Runtime, RuntimeError};
use engine_serialization::{GameObjectPlan, ScenePlan, TransformPlan};

fn create_owner(runtime: &mut Runtime, name: &str) -> (ObjectId, ObjectId) {
    let owner = runtime.create_gameobject(name).expect("create GameObject");
    let transform = runtime
        .gameobject_transform(owner)
        .expect("resolve mandatory Transform");
    (owner, transform)
}

fn assert_only_transform_is_published(runtime: &Runtime, owner: ObjectId, transform: ObjectId) {
    assert_eq!(runtime.physics_component_count(owner), Ok(1));
    assert_eq!(runtime.physics_component_at(owner, 0), Ok(transform));
}

fn single_owner_scene_plan() -> ScenePlan {
    ScenePlan {
        schema_version: 1,
        scene_name: String::from("physics-admission-staging"),
        roots: vec![2],
        game_objects: vec![GameObjectPlan {
            file_id: 1,
            name: String::from("StagedOwner"),
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

fn fault_lifecycle_dispatch(runtime: &mut Runtime) {
    runtime
        .physics_mode_write(1)
        .expect("select manual simulation");

    let (callback_owner, _) = create_owner(runtime, "faulting-callback-owner");
    let script = runtime
        .component_reserve(callback_owner, 0)
        .expect("reserve callback script");
    runtime
        .component_commit(script)
        .expect("publish callback script");

    runtime
        .lifecycle_begin_frame()
        .expect("begin lifecycle frame");
    runtime
        .lifecycle_begin_phase(1)
        .expect("begin first lifecycle phase");

    let delivery = runtime
        .lifecycle_next(1)
        .expect("request lifecycle callback")
        .expect("receive callback delivery");
    assert_eq!(delivery.component, script);

    runtime
        .lifecycle_ack(delivery.token, -6)
        .expect("record callback fault");
    assert_eq!(
        runtime.lifecycle_begin_phase(2),
        Err(RuntimeError::DispatchFaulted)
    );
}

#[test]
fn rejects_parented_and_child_bearing_owners_before_publication() {
    let mut runtime = Runtime::default();
    let (parent_owner, parent_transform) = create_owner(&mut runtime, "parent-owner");
    let (child_owner, child_transform) = create_owner(&mut runtime, "child-owner");

    runtime
        .transform_set_parent(child_transform, Some(parent_transform), false)
        .expect("parent the child Transform");

    assert_only_transform_is_published(&runtime, parent_owner, parent_transform);
    assert_only_transform_is_published(&runtime, child_owner, child_transform);

    assert_eq!(
        runtime.rigidbody_reserve(parent_owner),
        Err(RuntimeError::PhysicsUnsupported)
    );
    assert_eq!(
        runtime.rigidbody_reserve(child_owner),
        Err(RuntimeError::PhysicsUnsupported)
    );

    assert_only_transform_is_published(&runtime, parent_owner, parent_transform);
    assert_only_transform_is_published(&runtime, child_owner, child_transform);

    runtime
        .transform_set_parent(child_transform, None, false)
        .expect("restore the child owner to a supported root");
    let body = runtime
        .rigidbody_reserve(child_owner)
        .expect("retry admission after structural rejection");
    runtime
        .rigidbody_commit(body)
        .expect("publish body after structural repair");
    assert_eq!(runtime.physics_component_at(child_owner, 1), Ok(body));
}

#[test]
fn rejects_scale_and_inactive_owner_before_publication() {
    let mut runtime = Runtime::default();

    let (profile_owner, profile_transform) = create_owner(&mut runtime, "profile-owner");
    runtime
        .transform_write(profile_transform, 3, &[2.0, 1.0, 1.0])
        .expect("author unsupported scale");

    assert_eq!(
        runtime.rigidbody_reserve(profile_owner),
        Err(RuntimeError::PhysicsUnsupported)
    );
    assert_only_transform_is_published(&runtime, profile_owner, profile_transform);

    runtime
        .transform_write(profile_transform, 3, &[1.0, 1.0, 1.0])
        .expect("restore unit scale");
    runtime
        .gameobject_set_active(profile_owner, false)
        .expect("deactivate owner");

    assert_eq!(
        runtime.rigidbody_reserve(profile_owner),
        Err(RuntimeError::Lifecycle)
    );
    assert_only_transform_is_published(&runtime, profile_owner, profile_transform);

    runtime
        .gameobject_set_active(profile_owner, true)
        .expect("reactivate owner");
    let body = runtime
        .rigidbody_reserve(profile_owner)
        .expect("retry after rejected profile admissions");
    runtime
        .rigidbody_commit(body)
        .expect("publish body after valid retry");
    assert_eq!(runtime.physics_component_count(profile_owner), Ok(2));
    assert_eq!(runtime.physics_component_at(profile_owner, 1), Ok(body));
}

#[test]
fn rejects_pending_destroy_owner_before_publication() {
    let mut runtime = Runtime::default();
    let (owner, transform) = create_owner(&mut runtime, "pending-destroy-owner");

    runtime
        .object_destroy_deferred(owner)
        .expect("request deferred owner destruction");

    assert_eq!(
        runtime.rigidbody_reserve(owner),
        Err(RuntimeError::Lifecycle)
    );
    assert_only_transform_is_published(&runtime, owner, transform);

    let (independent_owner, _) = create_owner(&mut runtime, "independent-owner");
    let body = runtime
        .rigidbody_reserve(independent_owner)
        .expect("unrelated owner remains admissible");
    runtime
        .rigidbody_commit(body)
        .expect("publish independent body");
    assert_eq!(runtime.physics_component_at(independent_owner, 1), Ok(body));
    assert_only_transform_is_published(&runtime, owner, transform);
}

#[test]
fn rejects_attachment_when_lifecycle_dispatch_is_faulted() {
    let mut runtime = Runtime::default();
    let (owner, transform) = create_owner(&mut runtime, "healthy-owner");
    fault_lifecycle_dispatch(&mut runtime);

    assert_eq!(
        runtime.rigidbody_reserve(owner),
        Err(RuntimeError::DispatchFaulted)
    );
    assert_only_transform_is_published(&runtime, owner, transform);
}

#[test]
fn rejects_attachment_during_scene_staging_and_allows_retry_after_abort() {
    let mut runtime = Runtime::default();
    let (owner, transform) = create_owner(&mut runtime, "independent-owner");
    let scene = runtime
        .scene_stage(&single_owner_scene_plan())
        .expect("stage an unpublished scene");

    assert!(runtime.scene_staging());
    assert_eq!(
        runtime.rigidbody_reserve(owner),
        Err(RuntimeError::StagingBusy)
    );
    assert_only_transform_is_published(&runtime, owner, transform);

    runtime.scene_abort(scene).expect("abort staged scene");
    assert!(!runtime.scene_staging());

    let body = runtime
        .rigidbody_reserve(owner)
        .expect("retry after staging is cleared");
    runtime
        .rigidbody_commit(body)
        .expect("publish body after successful retry");
    assert_eq!(runtime.physics_component_count(owner), Ok(2));
    assert_eq!(runtime.physics_component_at(owner, 1), Ok(body));
}
