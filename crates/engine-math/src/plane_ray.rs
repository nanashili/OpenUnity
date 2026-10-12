//! Plane construction and ray intersection for finite three-dimensional vectors.

use crate::vector_arithmetic::scalar;
use crate::{MathError, Vector, finite, normalize_vector};

/// A plane represented by a unit normal and signed distance from the origin.
///
/// The plane equation is `dot(normal, point) + distance = 0`. The fields stay
/// private so every instance created through the public API preserves its
/// finite, normalized representation.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Plane {
    normal: Vector,
    distance: f32,
}

impl Plane {
    /// Construct a plane passing through `point` with the supplied normal.
    ///
    /// The normal is normalized before the signed distance is stored. A zero
    /// normal is unsupported because it does not define a plane.
    pub fn from_point_normal(point: Vector, normal: Vector) -> Result<Self, MathError> {
        finite(&point)?;
        finite(&normal)?;

        if is_zero(normal) {
            return Err(MathError::Unsupported);
        }

        let unit_normal = normalize_vector(normal)?;
        let distance = scalar(-dot_as_f64(unit_normal, point))?;

        Ok(Self {
            normal: unit_normal,
            distance,
        })
    }

    /// Return the plane's normalized normal.
    pub fn normal(&self) -> Vector {
        self.normal
    }

    /// Return the signed distance term in the plane equation.
    pub fn distance(&self) -> f32 {
        self.distance
    }

    /// Intersect a ray with this plane and return its nonnegative parameter.
    ///
    /// The supplied direction is used as-is. A result `t` identifies the
    /// point `origin + direction * t`; a negative result lies behind the ray
    /// origin and is therefore reported as no intersection. Parallel and
    /// coplanar rays also report no intersection.
    pub fn intersect_ray(
        &self,
        origin: Vector,
        direction: Vector,
    ) -> Result<Option<f32>, MathError> {
        finite(&origin)?;
        finite(&direction)?;

        if is_zero(direction) {
            return Err(MathError::Unsupported);
        }

        let denominator = dot_as_f64(self.normal, direction);
        if denominator == 0.0 {
            return Ok(None);
        }

        let numerator = -(dot_as_f64(self.normal, origin) + f64::from(self.distance));
        let parameter = numerator / denominator;

        if !parameter.is_finite() {
            return Err(MathError::Numeric);
        }

        if parameter < 0.0 {
            return Ok(None);
        }

        let result = parameter as f32;
        result
            .is_finite()
            .then_some(Some(result))
            .ok_or(MathError::Numeric)
    }
}

fn is_zero(value: Vector) -> bool {
    value.iter().all(|component| *component == 0.0)
}

fn dot_as_f64(a: Vector, b: Vector) -> f64 {
    a.iter()
        .zip(b.iter())
        .map(|(left, right)| f64::from(*left) * f64::from(*right))
        .sum()
}
