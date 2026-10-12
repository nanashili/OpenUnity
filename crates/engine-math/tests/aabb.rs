//! Conformance tests for the public AABB API.

use engine_math::{Aabb, MathError, Vector};

#[test]
fn aabb_preserves_valid_endpoints_and_includes_faces() {
    let minimum: Vector = [-2.0, 1.0, 4.0];
    let maximum: Vector = [3.0, 5.0, 8.0];
    let bounds = Aabb::try_new(minimum, maximum).expect("ordered finite bounds are valid");

    assert_eq!(bounds.min(), minimum);
    assert_eq!(bounds.max(), maximum);
    assert_eq!(bounds.contains(minimum), Ok(true));
    assert_eq!(bounds.contains(maximum), Ok(true));
    assert_eq!(bounds.contains([0.0, 3.0, 6.0]), Ok(true));
    assert_eq!(bounds.contains([3.0, 3.0, 9.0]), Ok(false));
}

#[test]
fn aabb_rejects_inverted_or_nonfinite_bounds() {
    assert_eq!(
        Aabb::try_new([0.0, 2.0, 0.0], [1.0, 1.0, 1.0]),
        Err(MathError::Unsupported)
    );
    assert_eq!(
        Aabb::try_new([0.0, f32::NAN, 0.0], [1.0, 1.0, 1.0]),
        Err(MathError::Numeric)
    );
    assert_eq!(
        Aabb::try_new([0.0, 0.0, 0.0], [1.0, f32::INFINITY, 1.0]),
        Err(MathError::Numeric)
    );
}

#[test]
fn aabb_rejects_nonfinite_containment_points() {
    let bounds = Aabb::try_new([-1.0, -1.0, -1.0], [1.0, 1.0, 1.0]).unwrap();

    assert_eq!(
        bounds.contains([0.0, f32::NAN, 0.0]),
        Err(MathError::Numeric)
    );
}

#[test]
fn aabb_union_selects_each_axis_endpoint() {
    let first = Aabb::try_new([-2.0, 1.0, 6.0], [3.0, 5.0, 8.0]).unwrap();
    let second = Aabb::try_new([-4.0, 2.0, 4.0], [1.0, 7.0, 10.0]).unwrap();

    let combined = first.union(&second);

    assert_eq!(combined.min(), [-4.0, 1.0, 4.0]);
    assert_eq!(combined.max(), [3.0, 7.0, 10.0]);
}
