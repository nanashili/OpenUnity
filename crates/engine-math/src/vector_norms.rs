//! Euclidean length and normalization for finite three-dimensional vectors.

use crate::{MathError, Vector, finite, vector};

/// Return the Euclidean length of a finite binary32 vector.
///
/// The norm is evaluated with binary64 intermediates after scaling by the
/// largest absolute component. This keeps the sum of squares bounded without
/// changing the binary32 public representation.
///
/// # Errors
/// Returns [`MathError::Numeric`] when an input is nonfinite, the vector is
/// zero, or the length cannot be represented as finite binary32.
pub fn length(value: Vector) -> Result<f32, MathError> {
    let norm = stable_length(value)?;
    let result = norm as f32;

    if result.is_finite() {
        Ok(result)
    } else {
        Err(MathError::Numeric)
    }
}

/// Return a unit vector derived from a finite binary32 vector.
///
/// Components are divided using the stable binary64 length, then converted
/// through the existing finite vector conversion policy.
///
/// # Errors
/// Returns [`MathError::Numeric`] when an input is nonfinite, the vector is
/// zero, or a normalized component cannot be represented as finite binary32.
pub fn normalize_vector(value: Vector) -> Result<Vector, MathError> {
    let norm = stable_length(value)?;
    let components = std::array::from_fn(|index| f64::from(value[index]) / norm);

    vector(components)
}

fn stable_length(value: Vector) -> Result<f64, MathError> {
    finite(&value)?;

    let scale = value
        .iter()
        .map(|component| f64::from(*component).abs())
        .fold(0.0, f64::max);

    if scale == 0.0 {
        return Err(MathError::Numeric);
    }

    let scaled_sum = value
        .iter()
        .map(|component| {
            let scaled = f64::from(*component) / scale;
            scaled * scaled
        })
        .sum::<f64>();
    let norm = scale * scaled_sum.sqrt();

    if norm.is_finite() && norm > 0.0 {
        Ok(norm)
    } else {
        Err(MathError::Numeric)
    }
}
