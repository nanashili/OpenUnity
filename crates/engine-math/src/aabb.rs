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
}

fn has_inverted_axis(min: Vector, max: Vector) -> bool {
    (0..3).any(|axis| min[axis] > max[axis])
}
