use super::{Runtime, RuntimeError, Scene};
use std::collections::{BTreeMap, BTreeSet};
use std::thread;

fn empty_scene() -> Scene {
    Scene {
        thread: thread::current().id(),
        local: BTreeMap::new(),
        ids: BTreeSet::new(),
        owners: Vec::new(),
        roots: Vec::new(),
        staged: None,
        destroy_requested: false,
    }
}

#[test]
fn invalid_scene_precedes_runtime_staging_status() {
    let mut runtime = Runtime::default();
    let missing_scene = runtime.scenes.handles.insert(empty_scene()).unwrap();
    runtime.scenes.handles.remove(missing_scene).unwrap();

    let live_scene = runtime.scenes.handles.insert(empty_scene()).unwrap();
    runtime.scenes.staging = Some(live_scene);

    assert_eq!(
        runtime.scene_snapshot_scene_ready(missing_scene),
        Err(RuntimeError::InvalidScene)
    );
    assert_eq!(
        runtime.scene_snapshot_scene_ready(live_scene),
        Err(RuntimeError::StagingBusy)
    );
}
