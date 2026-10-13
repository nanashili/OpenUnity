//! Quaternion normalization, Hamilton products, and vector rotation.

use crate::vector_arithmetic::scalar;
use crate::{MathError, Quaternion, Vector, finite, vector};

/// Normalize a quaternion using exact component squares and a binary64 length.
/// # Errors
/// Returns a numeric error for nonfinite components or a zero length.
pub fn normalize(q: Quaternion) -> Result<Quaternion, MathError> {
    finite(&q)?;
    let length = q
        .iter()
        .map(|value| {
            let component = f64::from(*value);
            component * component
        })
        .sum::<f64>()
        .sqrt();
    if length == 0.0 {
        return Err(MathError::Numeric);
    }
    Ok(q.map(|value| (f64::from(value) / length) as f32))
}

/// Return the quaternion conjugate.
pub fn conjugate(q: Quaternion) -> Quaternion {
    [-q[0], -q[1], -q[2], q[3]]
}

/// Compute and normalize a Hamilton quaternion product.
/// # Errors
/// Returns a numeric error for an invalid product or normalization.
pub fn multiply(a: Quaternion, b: Quaternion) -> Result<Quaternion, MathError> {
    let [x, y, z, w] = a.map(f64::from);
    let [u, v, s, t] = b.map(f64::from);
    normalize([
        scalar(w * u + x * t + y * s - z * v)?,
        scalar(w * v - x * s + y * t + z * u)?,
        scalar(w * s + x * v - y * u + z * t)?,
        scalar(w * t - x * u - y * v - z * s)?,
    ])
}

/// Apply a stored normalized quaternion to a vector, correcting its rounded norm.
/// # Errors
/// Returns a numeric error if a result cannot be represented as finite binary32.
pub fn rotate(q: Quaternion, point: Vector) -> Result<Vector, MathError> {
    let [x, y, z, w] = q.map(f64::from);
    let [a, b, c] = point.map(f64::from);
    // q is normalized when stored; divide by its actual squared norm to avoid
    // introducing scale from binary32 normalization rounding.
    let norm = x * x + y * y + z * z + w * w;
    vector([
        ((w * w + x * x - y * y - z * z) * a
            + 2.0 * (x * y - w * z) * b
            + 2.0 * (x * z + w * y) * c)
            / norm,
        (2.0 * (x * y + w * z) * a
            + (w * w - x * x + y * y - z * z) * b
            + 2.0 * (y * z - w * x) * c)
            / norm,
        (2.0 * (x * z - w * y) * a
            + 2.0 * (y * z + w * x) * b
            + (w * w - x * x - y * y + z * z) * c)
            / norm,
    ])
}
