//! Conformance tests for plane construction and ray hits.

use engine_math::{MathError, Plane, Vector};

#[test]
fn plane_normalizes_its_normal_and_stores_the_plane_equation() {
    let point: Vector = [0.0, 3.0, 0.0];
    let plane = Plane::from_point_normal(point, [0.0, 2.0, 0.0]).unwrap();

    assert_eq!(plane.normal(), [0.0, 1.0, 0.0]);
    assert_eq!(plane.distance(), -3.0);
}

#[test]
fn ray_intersection_returns_a_known_positive_parameter() {
    let plane = Plane::from_point_normal([0.0, 3.0, 0.0], [0.0, 2.0, 0.0]).unwrap();

    assert_eq!(
        plane.intersect_ray([0.0, 0.0, 0.0], [0.0, 1.0, 0.0]),
        Ok(Some(3.0))
    );
}

#[test]
fn ray_intersection_rejects_hits_behind_origin() {
    let plane = Plane::from_point_normal([0.0, 3.0, 0.0], [0.0, 1.0, 0.0]).unwrap();

    assert_eq!(
        plane.intersect_ray([0.0, 4.0, 0.0], [0.0, 1.0, 0.0]),
        Ok(None)
    );
}

#[test]
fn parallel_and_coplanar_rays_have_no_intersection() {
    let plane = Plane::from_point_normal([0.0, 3.0, 0.0], [0.0, 1.0, 0.0]).unwrap();

    assert_eq!(
        plane.intersect_ray([0.0, 0.0, 0.0], [1.0, 0.0, 0.0]),
        Ok(None)
    );
    assert_eq!(
        plane.intersect_ray([5.0, 3.0, 6.0], [1.0, 0.0, 0.0]),
        Ok(None)
    );
}

#[test]
fn plane_and_ray_reject_zero_vectors() {
    assert_eq!(
        Plane::from_point_normal([0.0, 0.0, 0.0], [0.0, 0.0, 0.0]),
        Err(MathError::Unsupported)
    );

    let plane = Plane::from_point_normal([0.0, 0.0, 0.0], [0.0, 1.0, 0.0]).unwrap();
    assert_eq!(
        plane.intersect_ray([0.0, 0.0, 0.0], [0.0, 0.0, 0.0]),
        Err(MathError::Unsupported)
    );
}

#[test]
fn plane_and_ray_reject_nonfinite_inputs() {
    assert_eq!(
        Plane::from_point_normal([f32::INFINITY, 0.0, 0.0], [0.0, 1.0, 0.0]),
        Err(MathError::Numeric)
    );

    let plane = Plane::from_point_normal([0.0, 0.0, 0.0], [0.0, 1.0, 0.0]).unwrap();
    assert_eq!(
        plane.intersect_ray([0.0, f32::NAN, 0.0], [1.0, 0.0, 0.0]),
        Err(MathError::Numeric)
    );
}
