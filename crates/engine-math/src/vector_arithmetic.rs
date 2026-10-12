//! Finite vector arithmetic and binary64-to-binary32 conversion.

use crate::{MathError, Vector};

pub(crate) fn scalar(value: f64) -> Result<f32, MathError> {
    let result = value as f32;
    result
        .is_finite()
        .then_some(result)
        .ok_or(MathError::Numeric)
}

/// Convert three binary64 components to finite binary32 components.
/// # Errors
/// Returns a numeric error if a converted component is not finite.
pub fn vector(values: [f64; 3]) -> Result<Vector, MathError> {
    Ok([scalar(values[0])?, scalar(values[1])?, scalar(values[2])?])
}

/// Validate that every supplied binary32 component is finite.
/// # Errors
/// Returns a numeric error for a nonfinite component.
pub fn finite(values: &[f32]) -> Result<(), MathError> {
    if values.iter().all(|value| value.is_finite()) {
        Ok(())
    } else {
        Err(MathError::Numeric)
    }
}

/// Add vector components using binary64 intermediates.
/// # Errors
/// Returns a numeric error if a result cannot be represented as finite binary32.
pub fn add(a: Vector, b: Vector) -> Result<Vector, MathError> {
    vector(std::array::from_fn(|i| f64::from(a[i]) + f64::from(b[i])))
}

/// Subtract vector components using binary64 intermediates.
/// # Errors
/// Returns a numeric error if a result cannot be represented as finite binary32.
pub fn subtract(a: Vector, b: Vector) -> Result<Vector, MathError> {
    vector(std::array::from_fn(|i| f64::from(a[i]) - f64::from(b[i])))
}

/// Multiply vector components using binary64 intermediates.
/// # Errors
/// Returns a numeric error if a result cannot be represented as finite binary32.
pub fn scale(a: Vector, b: Vector) -> Result<Vector, MathError> {
    vector(std::array::from_fn(|i| f64::from(a[i]) * f64::from(b[i])))
}

/// Divide vector components using binary64 intermediates.
/// # Errors
/// Returns unsupported for a zero divisor component, or a numeric input/result error.
pub fn divide(a: Vector, b: Vector) -> Result<Vector, MathError> {
    if b.contains(&0.0) {
        return Err(MathError::Unsupported);
    }

    finite(&a)?;
    finite(&b)?;

    vector(std::array::from_fn(|i| f64::from(a[i]) / f64::from(b[i])))
}
