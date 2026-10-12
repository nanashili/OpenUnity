//! Conformance tests for the public vector geometry API.
//!
//! These cases use hand-computed values.  They intentionally do not call
//! engine-math helpers to construct expected results.

use engine_math::{MathError, Vector, cross, dot, length, lerp, normalize_vector};

#[test]
fn dot_accumulates_components_with_the_admitted_order() {
    let left: Vector = [1.0, 2.0, 3.0];
    let right: Vector = [4.0, -5.0, 6.0];

    assert_eq!(dot(left, right), Ok(12.0));
}

#[test]
fn dot_uses_a_finite_binary32_result_boundary() {
    let overflowing_component: Vector = [f32::MAX, 0.0, 0.0];

    assert_eq!(
        dot(overflowing_component, overflowing_component),
        Err(MathError::Numeric)
    );
}

#[test]
fn dot_rejects_nonfinite_input_before_computation() {
    assert_eq!(
        dot([f32::NAN, 2.0, 3.0], [4.0, 5.0, 6.0]),
        Err(MathError::Numeric)
    );
}

#[test]
fn cross_returns_the_right_handed_determinant() {
    let left: Vector = [1.0, 2.0, 3.0];
    let right: Vector = [4.0, -5.0, 6.0];

    assert_eq!(cross(left, right), Ok([27.0, 6.0, -13.0]));
}

#[test]
fn cross_rejects_nonfinite_input_and_output() {
    assert_eq!(
        cross([1.0, f32::INFINITY, 3.0], [4.0, 5.0, 6.0]),
        Err(MathError::Numeric)
    );

    let overflowing_component: Vector = [f32::MAX, 0.0, 0.0];
    assert_eq!(
        cross(overflowing_component, [0.0, f32::MAX, 0.0]),
        Err(MathError::Numeric)
    );
}

#[test]
fn length_uses_the_euclidean_norm() {
    assert_eq!(length([3.0, 4.0, 12.0]), Ok(13.0));
}

#[test]
fn length_rejects_nonfinite_input_or_result() {
    assert_eq!(length([f32::NAN, 0.0, 0.0]), Err(MathError::Numeric));

    let large_vector: Vector = [f32::MAX, f32::MAX, f32::MAX];
    assert_eq!(length(large_vector), Err(MathError::Numeric));
}

#[test]
fn normalize_vector_returns_a_unit_direction() {
    assert_eq!(normalize_vector([0.0, 3.0, 4.0]), Ok([0.0, 0.6, 0.8]));
}

#[test]
fn normalize_vector_rejects_zero_and_nonfinite_vectors() {
    assert_eq!(normalize_vector([0.0, 0.0, 0.0]), Err(MathError::Numeric));
    assert_eq!(
        normalize_vector([f32::INFINITY, 0.0, 0.0]),
        Err(MathError::Numeric)
    );
}

#[test]
fn lerp_interpolates_with_a_bounded_parameter() {
    let start: Vector = [1.0, 2.0, 3.0];
    let end: Vector = [5.0, 6.0, 7.0];

    assert_eq!(lerp(start, end, 0.25), Ok([2.0, 3.0, 4.0]));
    assert_eq!(lerp(start, end, 0.0), Ok(start));
    assert_eq!(lerp(start, end, 1.0), Ok(end));
}

#[test]
fn lerp_handles_large_finite_endpoints_in_binary64() {
    let start: Vector = [f32::MAX, 0.0, 0.0];
    let end: Vector = [-f32::MAX, 0.0, 0.0];

    assert_eq!(lerp(start, end, 0.5), Ok([0.0, 0.0, 0.0]));
}

#[test]
fn lerp_rejects_nonfinite_inputs_and_out_of_range_parameters() {
    assert_eq!(
        lerp([f32::NAN, 0.0, 0.0], [1.0, 1.0, 1.0], 0.5),
        Err(MathError::Numeric)
    );
    assert_eq!(
        lerp([0.0, 0.0, 0.0], [1.0, 1.0, 1.0], f32::INFINITY),
        Err(MathError::Numeric)
    );

    for parameter in [-f32::EPSILON, -1.0, 1.0 + f32::EPSILON, 2.0] {
        assert_eq!(
            lerp([0.0, 0.0, 0.0], [1.0, 1.0, 1.0], parameter),
            Err(MathError::Unsupported)
        );
    }
}
