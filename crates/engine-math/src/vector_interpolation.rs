//! Bounded linear interpolation for finite three-component vectors.

use crate::{MathError, Vector, finite, vector};

/// Interpolate between two vectors at a bounded normalized position.
///
/// The interpolation parameter is inclusive at both endpoints, so `0.0`
/// returns `a` and `1.0` returns `b`. Computation uses binary64 intermediates
/// before the result is converted to the engine's binary32 vector type.
///
/// # Errors
///
/// Returns [`MathError::Numeric`] when an endpoint or parameter is nonfinite,
/// or when a computed component cannot be represented as finite binary32.
/// Returns [`MathError::Unsupported`] when `t` is outside the inclusive
/// normalized interval `[0.0, 1.0]`.
pub fn lerp(a: Vector, b: Vector, t: f32) -> Result<Vector, MathError> {
    validate_inputs(a, b, t)?;

    if !(0.0..=1.0).contains(&t) {
        return Err(MathError::Unsupported);
    }

    interpolate_components(a, b, t)
}

fn validate_inputs(a: Vector, b: Vector, t: f32) -> Result<(), MathError> {
    finite(&a)?;
    finite(&b)?;
    finite(&[t])
}

fn interpolate_components(a: Vector, b: Vector, t: f32) -> Result<Vector, MathError> {
    let parameter = f64::from(t);

    vector(std::array::from_fn(|index| {
        let start = f64::from(a[index]);
        let end = f64::from(b[index]);

        start + (end - start) * parameter
    }))
}
