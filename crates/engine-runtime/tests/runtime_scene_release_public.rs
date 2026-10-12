//! Public lifetime coverage for scene release and value-owned snapshots.

use engine_identifiers::ObjectId;
use engine_runtime::{Runtime, RuntimeError, SnapshotState};
use engine_serialization::{GameObjectPlan, ScenePlan, TransformPlan};

fn single_owner_scene() -> ScenePlan {
    ScenePlan {
        schema_version: 1,
        scene_name: String::from("public-release-lifetime"),
        roots: vec![2],
        game_objects: vec![GameObjectPlan {
            file_id: 1,
            name: String::from("Owner"),
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

fn publish_scene(runtime: &mut Runtime) -> ObjectId {
    let scene = runtime.scene_stage(&single_owner_scene()).unwrap();
    runtime.scene_publish(scene).unwrap();
    scene
}

#[test]
fn captured_snapshot_remains_readable_after_scene_release() {
    let mut runtime = Runtime::default();
    let scene = publish_scene(&mut runtime);
    let snapshot = runtime.scene_snapshot(scene, &[1]).unwrap();

    runtime.scene_release(scene).unwrap();

    let row = snapshot.row(0).unwrap();
    assert_eq!(snapshot.row_count(), 1);
    assert_eq!(row.local_file_id(), 1);
    assert_eq!(row.state(), SnapshotState::AliveActive);
    assert!(row.unit_scale_chain());
    assert_eq!(
        row.affine(),
        &[1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0]
    );
    assert_eq!(
        runtime.scene_snapshot(scene, &[1]).unwrap_err(),
        RuntimeError::InvalidScene
    );
}

#[test]
fn queued_scene_destruction_blocks_release_until_flush_then_snapshot_survives() {
    let mut runtime = Runtime::default();
    let scene = publish_scene(&mut runtime);
    let snapshot = runtime.scene_snapshot(scene, &[1]).unwrap();
    runtime.scene_destroy(scene).unwrap();

    assert_eq!(runtime.scene_release(scene), Err(RuntimeError::StagingBusy));

    runtime.lifecycle_flush_destroy().unwrap();
    assert!(runtime.lifecycle_next(1).unwrap().is_none());
    runtime.scene_release(scene).unwrap();

    assert_eq!(snapshot.row(0).unwrap().state(), SnapshotState::AliveActive);
}
