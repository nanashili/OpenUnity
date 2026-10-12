//! Independently authored affine and Hamilton quaternion mathematics.
use crate::RuntimeError;

pub(crate) type Vector = [f32; 3];
pub(crate) type Quaternion = [f32; 4];

fn scalar(value: f64) -> Result<f32, RuntimeError> {
    let result = value as f32;
    result
        .is_finite()
        .then_some(result)
        .ok_or(RuntimeError::Numeric)
}

pub(crate) fn vector(values: [f64; 3]) -> Result<Vector, RuntimeError> {
    Ok([scalar(values[0])?, scalar(values[1])?, scalar(values[2])?])
}

pub(crate) fn finite(values: &[f32]) -> Result<(), RuntimeError> {
    if values.iter().all(|value| value.is_finite()) {
        Ok(())
    } else {
        Err(RuntimeError::Numeric)
    }
}

pub(crate) fn normalize(q: Quaternion) -> Result<Quaternion, RuntimeError> {
    finite(&q)?;
    let length = q
        .iter()
        .map(|value| f64::from(*value).powi(2))
        .sum::<f64>()
        .sqrt();
    if length == 0.0 {
        return Err(RuntimeError::Numeric);
    }
    Ok(q.map(|value| (f64::from(value) / length) as f32))
}

pub(crate) fn conjugate(q: Quaternion) -> Quaternion {
    [-q[0], -q[1], -q[2], q[3]]
}

pub(crate) fn multiply(a: Quaternion, b: Quaternion) -> Result<Quaternion, RuntimeError> {
    let [x, y, z, w] = a.map(f64::from);
    let [u, v, s, t] = b.map(f64::from);
    normalize([
        scalar(w * u + x * t + y * s - z * v)?,
        scalar(w * v - x * s + y * t + z * u)?,
        scalar(w * s + x * v - y * u + z * t)?,
        scalar(w * t - x * u - y * v - z * s)?,
    ])
}

pub(crate) fn rotate(q: Quaternion, point: Vector) -> Result<Vector, RuntimeError> {
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

pub(crate) fn add(a: Vector, b: Vector) -> Result<Vector, RuntimeError> {
    vector(std::array::from_fn(|i| f64::from(a[i]) + f64::from(b[i])))
}
pub(crate) fn subtract(a: Vector, b: Vector) -> Result<Vector, RuntimeError> {
    vector(std::array::from_fn(|i| f64::from(a[i]) - f64::from(b[i])))
}
pub(crate) fn scale(a: Vector, b: Vector) -> Result<Vector, RuntimeError> {
    vector(std::array::from_fn(|i| f64::from(a[i]) * f64::from(b[i])))
}
pub(crate) fn divide(a: Vector, b: Vector) -> Result<Vector, RuntimeError> {
    if b.contains(&0.0) {
        return Err(RuntimeError::Unsupported);
    }
    vector(std::array::from_fn(|i| f64::from(a[i]) / f64::from(b[i])))
}
