use engine_math::Vector;

use super::FollowError;

pub(super) fn difference(a: Vector, b: Vector) -> [f64; 3] {
    std::array::from_fn(|axis| f64::from(a[axis]) - f64::from(b[axis]))
}

pub(super) fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

pub(super) fn cross_product(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

pub(super) fn segment_length(a: Vector, b: Vector) -> f64 {
    dot(difference(a, b), difference(a, b)).sqrt()
}

pub(super) fn interpolate(a: Vector, b: Vector, ratio: f64) -> Result<Vector, FollowError> {
    let values: [f64; 3] = std::array::from_fn(|axis| {
        f64::from(a[axis]) + (f64::from(b[axis]) - f64::from(a[axis])) * ratio
    });
    Ok([
        checked_binary32(values[0])?,
        checked_binary32(values[1])?,
        checked_binary32(values[2])?,
    ])
}

pub(super) fn checked_binary32(value: f64) -> Result<f32, FollowError> {
    let result = value as f32;
    if result.is_finite() {
        Ok(result)
    } else {
        Err(FollowError::StepOverflow)
    }
}
