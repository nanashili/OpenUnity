//! Dot and right-handed cross products for finite three-dimensional vectors.

use crate::vector_arithmetic::scalar;
use crate::{MathError, Vector, finite, vector};

/// Return the dot product of two finite binary32 vectors.
///
/// The three products and their sum use binary64 intermediates. The final
/// value is converted through the same finite binary32 policy as the other
/// vector operations.
///
/// # Errors
/// Returns [`MathError::Numeric`] when an input is nonfinite or when the
/// result cannot be represented as finite binary32.
pub fn dot(a: Vector, b: Vector) -> Result<f32, MathError> {
    finite(&a)?;
    finite(&b)?;

    let product = (0..a.len())
        .map(|index| f64::from(a[index]) * f64::from(b[index]))
        .sum::<f64>();

    scalar(product)
}

/// Return the right-handed cross product of two finite binary32 vectors.
///
/// Components are evaluated as binary64 determinants and converted through
/// the existing finite vector conversion policy. Components use `(x, y, z)`
/// order, so `cross([1, 0, 0], [0, 1, 0])` points along positive `z`.
///
/// # Errors
/// Returns [`MathError::Numeric`] when an input is nonfinite or when a
/// component cannot be represented as finite binary32.
pub fn cross(a: Vector, b: Vector) -> Result<Vector, MathError> {
    finite(&a)?;
    finite(&b)?;

    let components = [
        f64::from(a[1]) * f64::from(b[2]) - f64::from(a[2]) * f64::from(b[1]),
        f64::from(a[2]) * f64::from(b[0]) - f64::from(a[0]) * f64::from(b[2]),
        f64::from(a[0]) * f64::from(b[1]) - f64::from(a[1]) * f64::from(b[0]),
    ];

    vector(components)
}
