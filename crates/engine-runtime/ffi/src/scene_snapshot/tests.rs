use super::ou_scene_snapshot;
use crate::runtime_store::store;
use engine_object::ObjectId;
use engine_runtime::Runtime;
use engine_serialization::{GameObjectPlan, ScenePlan, TransformPlan};

fn staged_runtime() -> (ObjectId, ObjectId) {
    let mut runtime = Runtime::default();
    let scene_id = runtime
        .scene_stage(&single_owner_plan())
        .expect("valid scene plan should stage");

    let runtime_id = store()
        .lock()
        .expect("runtime store should be available")
        .insert(runtime)
        .expect("runtime identity should be available");

    (runtime_id, scene_id)
}

fn single_owner_plan() -> ScenePlan {
    ScenePlan {
        schema_version: 1,
        scene_name: String::from("snapshot-precedence"),
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
            local_position: [0.0; 3],
            local_rotation: [0.0, 0.0, 0.0, 1.0],
            local_scale: [1.0; 3],
        }],
        scripts: Vec::new(),
    }
}

#[test]
fn invalid_scene_precedes_staged_runtime_status() {
    let (runtime_id, staged_scene_id) = staged_runtime();
    let owner_file_ids = [1_i64];
    let mut output = [0xA5_u8; 80];

    // SAFETY: both buffers are valid, aligned, writable/readable, and non-overlapping.
    let status = unsafe {
        ou_scene_snapshot(
            runtime_id.to_raw(),
            0,
            owner_file_ids.as_ptr(),
            1,
            output.as_mut_ptr(),
            i32::try_from(output.len()).expect("test buffer length fits i32"),
        )
    };

    // SAFETY: these use the same valid, non-overlapping buffers as the first call.
    let staged_scene_status = unsafe {
        ou_scene_snapshot(
            runtime_id.to_raw(),
            staged_scene_id.to_raw(),
            owner_file_ids.as_ptr(),
            1,
            output.as_mut_ptr(),
            i32::try_from(output.len()).expect("test buffer length fits i32"),
        )
    };

    let removed = store()
        .lock()
        .expect("runtime store should be available")
        .remove(runtime_id);
    assert!(removed.is_some(), "test runtime should be removed");
    assert_eq!(status, -15, "invalid scene must precede staged status");
    assert_eq!(
        staged_scene_status, -17,
        "staged scene must retain busy status"
    );
    assert!(output.iter().all(|byte| *byte == 0xA5));
}

#[test]
fn buffer_shapes_are_checked_before_runtime_access() {
    let ids = [1_i64];
    let mut output = [0xA5_u8; 80];
    let input = ids.as_ptr();
    let destination = output.as_mut_ptr();

    for count in [-1, 0, 4098] {
        assert_eq!(
            super::validate_buffers(input, count, destination, 80),
            Err(-5)
        );
    }
    assert_eq!(super::validate_buffers(input, 1, destination, -1), Err(-5));
    assert_eq!(super::validate_buffers(input, 1, destination, 79), Err(-7));
    assert_eq!(
        super::validate_buffers(std::ptr::null(), 1, destination, 80),
        Err(-5)
    );
    assert_eq!(
        super::validate_buffers(input, 1, std::ptr::null_mut(), 80),
        Err(-5)
    );
    assert_eq!(output, [0xA5; 80]);
}

#[test]
fn buffer_ranges_reject_misalignment_overflow_and_overlap_without_dereferencing() {
    let ids = [1_i64];
    let mut output = [0xA5_u8; 80];
    let destination = output.as_mut_ptr();
    let misaligned = ids.as_ptr().cast::<u8>().wrapping_add(1).cast::<i64>();
    let overflowing_input = std::ptr::without_provenance::<i64>(usize::MAX - 7);
    let overflowing_output = std::ptr::without_provenance_mut::<u8>(usize::MAX - 39);

    assert_eq!(
        super::validate_buffers(misaligned, 1, destination, 80),
        Err(-5)
    );
    assert_eq!(
        super::validate_buffers(overflowing_input, 1, destination, 80),
        Err(-5)
    );
    assert_eq!(
        super::validate_buffers(ids.as_ptr(), 1, overflowing_output, 80),
        Err(-5)
    );
    assert_eq!(
        super::validate_buffers(ids.as_ptr(), 1, ids.as_ptr().cast_mut().cast(), 80),
        Err(-5)
    );
    assert_eq!(output, [0xA5; 80]);
}

#[test]
fn byte_output_needs_no_alignment_and_maximum_rows_have_exact_capacity() {
    let ids = [1_i64; 4097];
    let mut output = vec![0xA5_u8; 16 + 64 * ids.len() + 1];
    let destination = output.as_mut_ptr().wrapping_add(1);
    let capacity = i32::try_from(output.len() - 1).expect("bounded test capacity");

    assert_eq!(
        super::validate_buffers(ids.as_ptr(), 4097, destination, capacity),
        Ok(4097)
    );
    assert_eq!(
        super::validate_buffers(ids.as_ptr(), 4097, destination, capacity - 1),
        Err(-7)
    );
    assert!(output.iter().all(|byte| *byte == 0xA5));
}
