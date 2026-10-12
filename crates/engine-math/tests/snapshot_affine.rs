//! Independent acceptance vectors for the scene snapshot affine contract.
//!
//! Expected values are authored from matrix products and IEEE-754 conversion;
//! this file does not copy the candidate implementation or inspect its fields.

use engine_math::{MathError, SnapshotAffine3x4};

fn assert_wire_bits(actual: [f32; 12], expected: [u32; 12]) {
    assert_eq!(actual.map(f32::to_bits), expected);
}

#[test]
fn identity_has_exact_row_major_f32_bits() {
    let affine = SnapshotAffine3x4::identity();
    assert_wire_bits(
        affine.to_wire_rows().unwrap(),
        [
            0x3f800000, 0, 0, 0, 0, 0x3f800000, 0, 0, 0, 0, 0x3f800000, 0,
        ],
    );
}

#[test]
fn rotation_nonuniform_scale_and_parent_shear_preserve_the_product() {
    let parent =
        SnapshotAffine3x4::from_trs([1.0, 2.0, 3.0], [0.0, 0.0, 1.0, 0.0], [2.0, 1.0, 1.0])
            .unwrap();
    let local = SnapshotAffine3x4::from_trs(
        [4.0, 5.0, 6.0],
        [0.0, 0.0, 0.70710677, 0.70710677],
        [1.0, 3.0, 1.0],
    )
    .unwrap();

    let world = parent.compose(&local).unwrap();
    assert_wire_bits(
        world.to_wire_rows().unwrap(),
        [
            0x00000000, 0x40c00000, 0x00000000, 0xc0e00000, 0xbf800000, 0x00000000, 0x00000000,
            0xc0400000, 0x00000000, 0x00000000, 0x3f800000, 0x41100000,
        ],
    );
}

#[test]
fn stored_f32_quaternion_is_used_without_second_normalization() {
    // This is a valid f32 quaternion after Transform storage normalization.
    // Normalizing this already-rounded value again changes its final w bit.
    let affine = SnapshotAffine3x4::from_trs(
        [0.0, 0.0, 0.0],
        [
            f32::from_bits(0x3da0d8af),
            f32::from_bits(0xbf545dc1),
            f32::from_bits(0xbea7d44f),
            f32::from_bits(0x3ee3f406),
        ],
        [1.0, 1.0, 1.0],
    )
    .unwrap();

    assert_wire_bits(
        affine.to_wire_rows().unwrap(),
        [
            0xbf175a2d, 0x3e257421, 0xbf4a47e9, 0, 0xbed8286a, 0x3f45d42c, 0x3ef2a434, 0,
            0x3f2feb39, 0x3f1d2077, 0xbec6fea4, 0,
        ],
    );
}

#[test]
fn long_chain_retains_f64_intermediates_until_one_wire_quantization() {
    let mut accumulated = SnapshotAffine3x4::identity();
    for index in 1..=257 {
        let translation = index as f32 * 0.125;
        let local = SnapshotAffine3x4::from_trs(
            [translation, -translation, 0.0625],
            [0.0, 0.0, 0.0, 1.0],
            [1.0001, 0.9999, 1.0002],
        )
        .unwrap();
        accumulated = accumulated.compose(&local).unwrap();
    }

    assert_wire_bits(
        accumulated.to_wire_rows().unwrap(),
        [
            0x3f835527, 0x00000000, 0x00000000, 0x4583bc5b, 0x00000000, 0x3f7980d4, 0x00000000,
            0xc57ea0f1, 0x00000000, 0x00000000, 0x3f86c06e, 0x4183d8c8,
        ],
    );
}

#[test]
fn zero_and_reflection_scale_are_preserved() {
    let affine =
        SnapshotAffine3x4::from_trs([2.0, -3.0, 4.0], [0.0, 0.0, 0.0, 1.0], [-2.0, 0.0, 3.0])
            .unwrap();

    assert_wire_bits(
        affine.to_wire_rows().unwrap(),
        [
            0xc0000000, 0, 0, 0x40000000, 0x80000000, 0, 0, 0xc0400000, 0x80000000, 0, 0x40400000,
            0x40800000,
        ],
    );

    let negative_parent =
        SnapshotAffine3x4::from_trs([0.0, 0.0, 0.0], [0.0, 0.0, 0.0, 1.0], [-1.0, -1.0, -1.0])
            .unwrap();
    let zero_local =
        SnapshotAffine3x4::from_trs([0.0, 0.0, 0.0], [0.0, 0.0, 0.0, 1.0], [0.0, 0.0, 0.0])
            .unwrap();

    assert_wire_bits(
        negative_parent
            .compose(&zero_local)
            .unwrap()
            .to_wire_rows()
            .unwrap(),
        [
            0x80000000, 0x80000000, 0x80000000, 0, 0x80000000, 0x80000000, 0x80000000, 0,
            0x80000000, 0x80000000, 0x80000000, 0,
        ],
    );
}

#[test]
fn final_f32_overflow_is_numeric_error() {
    let parent = SnapshotAffine3x4::from_trs(
        [f32::MAX, 0.0, 0.0],
        [0.0, 0.0, 0.0, 1.0],
        [f32::MAX, 1.0, 1.0],
    )
    .unwrap();
    let local = SnapshotAffine3x4::from_trs([2.0, 0.0, 0.0], [0.0, 0.0, 0.0, 1.0], [2.0, 1.0, 1.0])
        .unwrap();

    assert_eq!(
        parent.compose(&local).unwrap().to_wire_rows(),
        Err(MathError::Numeric)
    );
}

#[test]
fn nonfinite_accumulation_is_rejected_before_wire_conversion() {
    let mut accumulated = SnapshotAffine3x4::from_trs(
        [0.0, 0.0, 0.0],
        [0.0, 0.0, 0.0, 1.0],
        [f32::MAX, f32::MAX, f32::MAX],
    )
    .unwrap();
    let local = accumulated;

    for _ in 0..256 {
        accumulated = match accumulated.compose(&local) {
            Ok(value) => value,
            Err(MathError::Numeric) => return,
            Err(error) => panic!("unexpected error: {error:?}"),
        };
    }

    panic!("finite f64 accumulation never reached the required error");
}
