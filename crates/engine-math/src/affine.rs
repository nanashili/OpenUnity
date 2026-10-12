//! Finite row-major affine transforms for points in three dimensions.
//!
//! This matrix primitive remains separate from runtime Transform storage.

use crate::vector_arithmetic::scalar;
use crate::{MathError, Quaternion, Vector, finite, normalize};

/// A finite affine transform with three row-major rows of four components.
///
/// The implicit fourth row is `[0, 0, 0, 1]`. A point is treated as the
/// column vector `[x, y, z, 1]`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Affine3x4 {
    rows: [[f32; 4]; 3],
}

impl Affine3x4 {
    /// Construct an affine transform from row-major components.
    ///
    /// The rows use the layout `[m00, m01, m02, m03]`, followed by rows one
    /// and two. The final column is the translation.
    pub fn from_rows(rows: [[f32; 4]; 3]) -> Result<Self, MathError> {
        for row in rows {
            finite(&row)?;
        }

        Ok(Self { rows })
    }

    /// Construct `T * R * S` from a position, normalized quaternion, and
    /// componentwise scale. Zero scale components are valid.
    ///
    /// The quaternion follows the existing engine-math normalization policy:
    /// nonfinite input and zero length return [`MathError::Numeric`].
    pub fn from_trs(
        position: Vector,
        rotation: Quaternion,
        scale: Vector,
    ) -> Result<Self, MathError> {
        finite(&position)?;
        finite(&scale)?;
        let rotation = normalize(rotation)?;
        let matrix = rotation_rows(rotation);

        let rows = std::array::from_fn(|row| {
            [
                scalar(matrix[row][0] * f64::from(scale[0])),
                scalar(matrix[row][1] * f64::from(scale[1])),
                scalar(matrix[row][2] * f64::from(scale[2])),
                Ok(position[row]),
            ]
        });

        Ok(Self {
            rows: resolve_rows(rows)?,
        })
    }

    /// Compose `self * local`, retaining nonuniform-scale shear in f64.
    pub fn compose(&self, local: &Self) -> Result<Self, MathError> {
        let rows = std::array::from_fn(|row| {
            std::array::from_fn(|column| {
                let value = (0..3)
                    .map(|index| {
                        f64::from(self.rows[row][index]) * f64::from(local.rows[index][column])
                    })
                    .sum::<f64>();
                if column == 3 {
                    scalar(value + f64::from(self.rows[row][3]))
                } else {
                    scalar(value)
                }
            })
        });

        Ok(Self {
            rows: resolve_rows(rows)?,
        })
    }

    /// Transform a finite point using the affine rows.
    pub fn transform_point(&self, point: Vector) -> Result<Vector, MathError> {
        finite(&point)?;

        let result = std::array::from_fn(|row| {
            let value = (0..3)
                .map(|column| f64::from(self.rows[row][column]) * f64::from(point[column]))
                .sum::<f64>();
            scalar(value + f64::from(self.rows[row][3]))
        });

        resolve_vector(result)
    }

    /// Return the inverse affine transform.
    ///
    /// An exactly zero linear determinant maps to [`MathError::Unsupported`].
    /// Nonfinite or unrepresentable arithmetic maps to
    /// [`MathError::Numeric`].
    pub fn inverse(&self) -> Result<Self, MathError> {
        let determinant = determinant(self.rows);
        if !determinant.is_finite() {
            return Err(MathError::Numeric);
        }
        if determinant == 0.0 {
            return Err(MathError::Unsupported);
        }

        let inverse_linear = inverse_linear(self.rows, determinant);
        let translation: [Result<f32, MathError>; 3] = std::array::from_fn(|row| {
            let value = (0..3)
                .map(|column| inverse_linear[row][column] * -f64::from(self.rows[column][3]))
                .sum::<f64>();
            scalar(value)
        });

        let rows = std::array::from_fn(|row| {
            [
                scalar(inverse_linear[row][0]),
                scalar(inverse_linear[row][1]),
                scalar(inverse_linear[row][2]),
                translation[row],
            ]
        });

        Ok(Self {
            rows: resolve_rows(rows)?,
        })
    }
}

fn resolve_rows(rows: [[Result<f32, MathError>; 4]; 3]) -> Result<[[f32; 4]; 3], MathError> {
    Ok([
        resolve_row(rows[0])?,
        resolve_row(rows[1])?,
        resolve_row(rows[2])?,
    ])
}

fn resolve_row(row: [Result<f32, MathError>; 4]) -> Result<[f32; 4], MathError> {
    Ok([row[0]?, row[1]?, row[2]?, row[3]?])
}

fn resolve_vector(vector: [Result<f32, MathError>; 3]) -> Result<Vector, MathError> {
    Ok([vector[0]?, vector[1]?, vector[2]?])
}

fn rotation_rows(rotation: Quaternion) -> [[f64; 3]; 3] {
    let [x, y, z, w] = rotation.map(f64::from);
    let norm = x * x + y * y + z * z + w * w;

    [
        [
            (w * w + x * x - y * y - z * z) / norm,
            2.0 * (x * y - w * z) / norm,
            2.0 * (x * z + w * y) / norm,
        ],
        [
            2.0 * (x * y + w * z) / norm,
            (w * w - x * x + y * y - z * z) / norm,
            2.0 * (y * z - w * x) / norm,
        ],
        [
            2.0 * (x * z - w * y) / norm,
            2.0 * (y * z + w * x) / norm,
            (w * w - x * x - y * y + z * z) / norm,
        ],
    ]
}

fn determinant(rows: [[f32; 4]; 3]) -> f64 {
    let a = rows[0].map(f64::from);
    let b = rows[1].map(f64::from);
    let c = rows[2].map(f64::from);

    a[0] * (b[1] * c[2] - b[2] * c[1]) - a[1] * (b[0] * c[2] - b[2] * c[0])
        + a[2] * (b[0] * c[1] - b[1] * c[0])
}

fn inverse_linear(rows: [[f32; 4]; 3], determinant: f64) -> [[f64; 3]; 3] {
    let a = rows[0].map(f64::from);
    let b = rows[1].map(f64::from);
    let c = rows[2].map(f64::from);

    [
        [
            (b[1] * c[2] - b[2] * c[1]) / determinant,
            (a[2] * c[1] - a[1] * c[2]) / determinant,
            (a[1] * b[2] - a[2] * b[1]) / determinant,
        ],
        [
            (b[2] * c[0] - b[0] * c[2]) / determinant,
            (a[0] * c[2] - a[2] * c[0]) / determinant,
            (a[2] * b[0] - a[0] * b[2]) / determinant,
        ],
        [
            (b[0] * c[1] - b[1] * c[0]) / determinant,
            (a[1] * c[0] - a[0] * c[1]) / determinant,
            (a[0] * b[1] - a[1] * b[0]) / determinant,
        ],
    ]
}
