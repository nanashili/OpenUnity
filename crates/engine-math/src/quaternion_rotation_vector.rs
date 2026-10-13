//! Bounded rotation-vector conversion for feature INFRA-MATH-ROTATION-VECTOR-001.

use crate::vector_arithmetic::scalar;
use crate::vector_norms::stable_length;
use crate::{MathError, Quaternion, Vector, finite, normalize};

const MAX_ROTATION_ANGLE: f64 = 1_048_576.0;

/// Convert an axis-angle rotation vector into a normalized quaternion.
///
/// The vector direction is the rotation axis and its Euclidean magnitude is
/// the angle in radians. Quaternion signs are preserved from the formula; the
/// result is not hemisphere-canonicalized.
///
/// # Errors
/// Returns [`MathError::Numeric`] for nonfinite inputs or when the computed
/// vector length exceeds the supported angle bound of `2^20` radians.
pub fn from_rotation_vector(value: Vector) -> Result<Quaternion, MathError> {
    finite(&value)?;

    if is_zero_vector(value) {
        return Ok([0.0, 0.0, 0.0, 1.0]);
    }

    let angle = stable_length(value)?;
    validate_angle(angle)?;

    let half_angle = angle / 2.0;
    let axis_scale = half_angle.sin() / angle;
    let [x, y, z] = value;

    let quaternion = [
        scalar(f64::from(x) * axis_scale)?,
        scalar(f64::from(y) * axis_scale)?,
        scalar(f64::from(z) * axis_scale)?,
        scalar(half_angle.cos())?,
    ];

    normalize(quaternion)
}

fn is_zero_vector(value: Vector) -> bool {
    value.iter().all(|component| *component == 0.0)
}

fn validate_angle(angle: f64) -> Result<(), MathError> {
    if angle.is_finite() && angle <= MAX_ROTATION_ANGLE {
        Ok(())
    } else {
        Err(MathError::Numeric)
    }
}
