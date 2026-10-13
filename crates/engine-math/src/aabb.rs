//! Finite axis-aligned bounds for three-dimensional vectors.

use crate::{MathError, Vector, finite};

/// A closed, finite axis-aligned box in three-dimensional space.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Aabb {
    min: Vector,
    max: Vector,
}

impl Aabb {
    /// Construct a box after validating finite, componentwise ordered bounds.
    ///
    /// # Errors
    ///
    /// Returns [`MathError::Numeric`] when either endpoint contains a
    /// nonfinite component. Returns [`MathError::Unsupported`] when any
    /// minimum component is greater than its corresponding maximum.
    pub fn try_new(min: Vector, max: Vector) -> Result<Self, MathError> {
        finite(&min)?;
        finite(&max)?;

        if has_inverted_axis(min, max) {
            return Err(MathError::Unsupported);
        }

        Ok(Self { min, max })
    }

    /// Return the minimum corner as a copied vector.
    pub fn min(&self) -> Vector {
        self.min
    }

    /// Return the maximum corner as a copied vector.
    pub fn max(&self) -> Vector {
        self.max
    }

    /// Test whether a finite point lies inside this closed box.
    ///
    /// Boundary points are included on every axis.
    ///
    /// # Errors
    ///
    /// Returns [`MathError::Numeric`] when `point` contains a nonfinite
    /// component.
    pub fn contains(&self, point: Vector) -> Result<bool, MathError> {
        finite(&point)?;

        Ok((0..3).all(|axis| self.min[axis] <= point[axis] && point[axis] <= self.max[axis]))
    }

    /// Return the smallest closed box containing both valid input boxes.
    pub fn union(&self, other: &Self) -> Self {
        let min = std::array::from_fn(|axis| self.min[axis].min(other.min[axis]));
        let max = std::array::from_fn(|axis| self.max[axis].max(other.max[axis]));

        Self { min, max }
    }

    /// Test whether two closed boxes overlap, including boundary contact.
    pub fn intersects(&self, other: &Self) -> bool {
        (0..3).all(|axis| self.min[axis] <= other.max[axis] && other.min[axis] <= self.max[axis])
    }

    /// Intersect a closed segment with this box using binary64 intermediates.
    ///
    /// Returns the least binary64 parameter `t` and its validated binary32
    /// point for `origin + t * displacement`, where `0 <= t <= 1`. Boundary
    /// contact and an origin inside the box both return the earliest hit.
    ///
    /// # Errors
    ///
    /// Returns [`MathError::Unsupported`] for zero displacement and
    /// [`MathError::Numeric`] for nonfinite inputs or unrepresentable output.
    pub fn intersect_segment(
        &self,
        origin: Vector,
        displacement: Vector,
    ) -> Result<Option<(f64, Vector)>, MathError> {
        finite(&origin)?;
        finite(&displacement)?;
        if displacement == [0.0; 3] {
            return Err(MathError::Unsupported);
        }

        let mut entry = 0.0_f64;
        let mut exit = 1.0_f64;
        for axis in 0..3 {
            let start = f64::from(origin[axis]);
            let delta = f64::from(displacement[axis]);
            let minimum = f64::from(self.min[axis]);
            let maximum = f64::from(self.max[axis]);

            if delta == 0.0 {
                if start < minimum || start > maximum {
                    return Ok(None);
                }
                continue;
            }

            let mut near = (minimum - start) / delta;
            let mut far = (maximum - start) / delta;
            if near > far {
                std::mem::swap(&mut near, &mut far);
            }
            if !near.is_finite() || !far.is_finite() {
                return Err(MathError::Numeric);
            }

            entry = entry.max(near);
            exit = exit.min(far);
            if entry > exit {
                return Ok(None);
            }
        }

        if !entry.is_finite() || !exit.is_finite() {
            return Err(MathError::Numeric);
        }
        let point = std::array::from_fn(|axis| {
            f64::from(origin[axis]) + entry * f64::from(displacement[axis])
        });
        if point.iter().any(|component| !component.is_finite()) {
            return Err(MathError::Numeric);
        }

        let point = point.map(|component| component as f32);
        if point.iter().any(|component| !component.is_finite()) {
            return Err(MathError::Numeric);
        }

        Ok(Some((entry, point)))
    }
}

fn has_inverted_axis(min: Vector, max: Vector) -> bool {
    (0..3).any(|axis| min[axis] > max[axis])
}
