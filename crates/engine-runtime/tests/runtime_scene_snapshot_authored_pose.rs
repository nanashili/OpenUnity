//! Public Runtime coverage for authored-pose snapshot retention across attachment.
//!
//! Audit:
//! - Behavior: a fresh scene snapshot keeps the Transform's authored qT after
//!   Rigidbody attachment, while a prior value-owned snapshot keeps its rows.
//! - Regression: attachment copies normalized qB back into Transform, or a
//!   prior snapshot aliases row storage changed during attachment.
//! - Coverage gap: existing public snapshot coverage checks scene release, not
//!   Rigidbody attachment. The Runtime snapshot boundary owns this behavior.
//! - Seams: the case uses only public scene, snapshot, and Rigidbody APIs.
//!   Its synthetic quaternion exercises OpenUnity policy; it is not a Unity
//!   compatibility fixture or an assertion about hidden Rigidbody state.

use engine_identifiers::ObjectId;
use engine_runtime::Runtime;
use engine_serialization::{GameObjectPlan, ScenePlan, TransformPlan};

const EXPECTED_AUTHORED_AFFINE_BITS: [u32; 12] = [
    0x0000_0000,
    0xbf72_2983,
    0x3ea6_0dd7,
    0x4000_0000,
    0x3f80_0000,
    0x0000_0000,
    0x0000_0000,
    0xbf80_0000,
    0x0000_0000,
    0x3ea6_0dd7,
    0x3f72_2983,
    0x4040_0000,
];

fn authored_pose_scene() -> ScenePlan {
    ScenePlan {
        schema_version: 1,
        scene_name: String::from("snapshot-authored-pose"),
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
            local_position: [2.0, -1.0, 3.0],
            local_rotation: [1.0, 1.0, 6.0, 6.0],
            local_scale: [1.0, 1.0, 1.0],
        }],
        scripts: Vec::new(),
    }
}

fn publish_authored_pose_scene(runtime: &mut Runtime) -> (ObjectId, ObjectId) {
    let scene = runtime
        .scene_stage(&authored_pose_scene())
        .expect("stage the authored-pose scene");
    runtime
        .scene_publish(scene)
        .expect("publish the authored-pose scene");
    let owner = runtime
        .scene_object(scene, 1)
        .expect("resolve the scene owner");

    (scene, owner)
}

#[test]
fn scene_snapshot_retains_authored_rotation_and_prior_rows_across_attachment() {
    let mut runtime = Runtime::default();
    let (scene, owner) = publish_authored_pose_scene(&mut runtime);

    let prior_snapshot = runtime
        .scene_snapshot(scene, &[1])
        .expect("capture the authored pose before attachment");
    let prior_affine = *prior_snapshot
        .row(0)
        .expect("capture one owner row")
        .affine();
    assert_eq!(affine_bits(&prior_affine), EXPECTED_AUTHORED_AFFINE_BITS);

    let body = runtime
        .rigidbody_reserve(owner)
        .expect("reserve a body for the authored-pose owner");
    runtime
        .rigidbody_commit(body)
        .expect("commit the body attachment");

    let current_snapshot = runtime
        .scene_snapshot(scene, &[1])
        .expect("capture a fresh pose after attachment");
    let current_affine = current_snapshot
        .row(0)
        .expect("capture the current owner row")
        .affine();

    assert_eq!(affine_bits(current_affine), EXPECTED_AUTHORED_AFFINE_BITS);
    assert_eq!(
        affine_bits(
            prior_snapshot
                .row(0)
                .expect("retain the pre-attachment snapshot row")
                .affine()
        ),
        affine_bits(&prior_affine)
    );
}

fn affine_bits(affine: &[f32; 12]) -> [u32; 12] {
    (*affine).map(f32::to_bits)
}
