//! Bounded shortest-arc interpolation for engine quaternions.

use crate::vector_arithmetic::scalar;
use crate::{MathError, Quaternion, finite, normalize};

/// Dot-product threshold at which spherical interpolation becomes ill
/// conditioned and normalized linear interpolation is used instead.
const NEAR_PARALLEL_DOT: f64 = 1.0 - 1e-6;

/// Interpolate two quaternions along their shortest normalized arc.
///
/// Both endpoints are normalized before interpolation. If their normalized
/// dot product is negative, the second endpoint is negated so the selected
/// arc has a nonnegative dot product. For a dot product at or above
/// [`NEAR_PARALLEL_DOT`], normalized linear interpolation avoids division by
/// a small sine. The hemisphere-adjusted endpoint is used at `t == 1.0`;
/// this preserves continuity of the selected shortest arc when the inputs
/// use opposite quaternion signs for the same rotation.
///
/// # Errors
/// Returns [`MathError::Numeric`] when an input, normalized endpoint, or
/// output is nonfinite, or when an endpoint has zero length. Returns
/// [`MathError::Unsupported`] when `t` is outside the inclusive unit interval.
pub fn slerp(a: Quaternion, b: Quaternion, t: f32) -> Result<Quaternion, MathError> {
    validate_parameter(t)?;

    let start = normalize(a)?;
    let mut end = normalize(b)?;
    let mut dot = quaternion_dot(start, end);

    if dot < 0.0 {
        end = negate(end);
        dot = -dot;
    }

    if dot >= NEAR_PARALLEL_DOT {
        return normalized_linear_interpolation(start, end, t);
    }

    spherical_interpolation(start, end, dot, t)
}

fn validate_parameter(t: f32) -> Result<(), MathError> {
    finite(&[t])?;

    if (0.0..=1.0).contains(&t) {
        Ok(())
    } else {
        Err(MathError::Unsupported)
    }
}

fn quaternion_dot(a: Quaternion, b: Quaternion) -> f64 {
    a.iter()
        .zip(b.iter())
        .map(|(left, right)| f64::from(*left) * f64::from(*right))
        .sum()
}

fn negate(q: Quaternion) -> Quaternion {
    q.map(|component| -component)
}

fn normalized_linear_interpolation(
    start: Quaternion,
    end: Quaternion,
    t: f32,
) -> Result<Quaternion, MathError> {
    let t = f64::from(t);
    let values: [Result<f32, MathError>; 4] = std::array::from_fn(|index| {
        let start = f64::from(start[index]);
        let end = f64::from(end[index]);
        scalar(start + (end - start) * t)
    });

    let [x, y, z, w] = values;
    normalize([x?, y?, z?, w?])
}

fn spherical_interpolation(
    start: Quaternion,
    end: Quaternion,
    dot: f64,
    t: f32,
) -> Result<Quaternion, MathError> {
    let angle = dot.clamp(-1.0, 1.0).acos();
    let sine = angle.sin();
    let t = f64::from(t);
    let start_weight = ((1.0 - t) * angle).sin() / sine;
    let end_weight = (t * angle).sin() / sine;

    let values: [Result<f32, MathError>; 4] = std::array::from_fn(|index| {
        let start = f64::from(start[index]);
        let end = f64::from(end[index]);
        scalar(start * start_weight + end * end_weight)
    });

    let [x, y, z, w] = values;
    normalize([x?, y?, z?, w?])
}
