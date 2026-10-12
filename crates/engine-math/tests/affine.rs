//! Conformance tests for the public affine transform API.

use engine_math::{Affine3x4, MathError, Quaternion, Vector};

const Z_QUARTER: Quaternion = [0.0, 0.0, 0.70710677, 0.70710677];
const Z_HALF_QUARTER: Quaternion = [0.0, 0.0, 0.38268343, 0.9238795];

fn assert_vector_close(actual: Vector, expected: Vector) {
    for (actual_component, expected_component) in actual.iter().zip(expected) {
        assert!((actual_component - expected_component).abs() < 2e-5);
    }
}

#[test]
fn local_trs_maps_a_point_after_scale_rotation_and_translation() {
    let transform =
        Affine3x4::from_trs([10.0, 20.0, 30.0], [0.0, 0.0, 0.0, 1.0], [2.0, 3.0, 4.0]).unwrap();

    assert_vector_close(
        transform.transform_point([1.0, -2.0, 0.5]).unwrap(),
        [12.0, 14.0, 32.0],
    );
}

#[test]
fn composition_preserves_nonuniform_scale_and_rotation_shear() {
    let parent = Affine3x4::from_trs([10.0, 0.0, 0.0], Z_QUARTER, [2.0, 1.0, 1.0]).unwrap();
    let local = Affine3x4::from_trs([1.0, 2.0, 0.0], Z_HALF_QUARTER, [1.0, 3.0, 1.0]).unwrap();

    let world = parent.compose(&local).unwrap();

    assert_vector_close(
        world.transform_point([1.0, 1.0, 0.0]).unwrap(),
        [5.1715727, -0.8284271, 0.0],
    );
}

#[test]
fn inverse_round_trip_recovers_the_original_point() {
    let transform = Affine3x4::from_trs([4.0, -3.0, 2.0], Z_HALF_QUARTER, [2.0, 3.0, 0.5]).unwrap();
    let inverse = transform.inverse().unwrap();
    let original = [1.5, -2.0, 4.0];
    let transformed = transform.transform_point(original).unwrap();

    assert_vector_close(inverse.transform_point(transformed).unwrap(), original);
}

#[test]
fn inverse_rejects_a_singular_linear_part() {
    let singular = Affine3x4::from_rows([
        [1.0, 0.0, 0.0, 2.0],
        [0.0, 0.0, 0.0, 3.0],
        [0.0, 0.0, 1.0, 4.0],
    ])
    .unwrap();

    assert_eq!(singular.inverse(), Err(MathError::Unsupported));
}

#[test]
fn affine_construction_and_results_reject_nonfinite_values() {
    assert_eq!(
        Affine3x4::from_rows([
            [1.0, 0.0, 0.0, 0.0],
            [0.0, f32::NAN, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
        ]),
        Err(MathError::Numeric)
    );

    let overflowing = Affine3x4::from_rows([
        [f32::MAX, 0.0, 0.0, 0.0],
        [0.0, 1.0, 0.0, 0.0],
        [0.0, 0.0, 1.0, 0.0],
    ])
    .unwrap();

    assert_eq!(
        overflowing.transform_point([2.0, 0.0, 0.0]),
        Err(MathError::Numeric)
    );
}
