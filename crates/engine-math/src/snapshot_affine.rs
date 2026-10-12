//! Binary64 affine accumulation for runtime scene snapshot publication.

use crate::{MathError, Quaternion, Vector, finite, rotate};

const BASIS: [Vector; 3] = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];

/// A three-dimensional affine snapshot retained in binary64 between writes.
///
/// Rows use `[m00, m01, m02, tx]` and the implicit fourth row is
/// `[0, 0, 0, 1]`. The linear part is stored in the same row-major layout as
/// the runtime wire representation.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SnapshotAffine3x4 {
    rows: [[f64; 4]; 3],
}

impl SnapshotAffine3x4 {
    /// Return the identity affine transform.
    pub fn identity() -> Self {
        Self {
            rows: [
                [1.0, 0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
            ],
        }
    }

    /// Construct a local snapshot from position, rotation, and scale.
    ///
    /// Rotation is evaluated through the existing f32 `rotate` operation for
    /// each unit basis vector before those results are widened to f64.
    pub fn from_trs(
        position: Vector,
        rotation: Quaternion,
        scale: Vector,
    ) -> Result<Self, MathError> {
        finite(&position)?;
        finite(&rotation)?;
        finite(&scale)?;

        let basis = BASIS.map(|axis| rotate(rotation, axis));
        let basis = [basis[0]?, basis[1]?, basis[2]?];
        let rows = std::array::from_fn(|row| {
            [
                checked_f64(f64::from(basis[0][row]) * f64::from(scale[0])),
                checked_f64(f64::from(basis[1][row]) * f64::from(scale[1])),
                checked_f64(f64::from(basis[2][row]) * f64::from(scale[2])),
                checked_f64(f64::from(position[row])),
            ]
        });

        resolve_rows(rows)
    }

    /// Compose `self * local`, preserving shear and all intermediate f64 data.
    pub fn compose(&self, local: &Self) -> Result<Self, MathError> {
        let rows = std::array::from_fn(|row| {
            std::array::from_fn(|column| {
                let first = self.rows[row][0] * local.rows[0][column];
                let second = self.rows[row][1] * local.rows[1][column];
                let third = self.rows[row][2] * local.rows[2][column];
                let linear = first + second + third;
                let value = if column == 3 {
                    linear + self.rows[row][3]
                } else {
                    linear
                };
                checked_f64(value)
            })
        });

        resolve_rows(rows)
    }

    /// Convert the snapshot to row-major binary32 wire data.
    pub fn to_wire_rows(&self) -> Result<[f32; 12], MathError> {
        let mut wire = [0.0; 12];
        for (index, value) in self.rows.into_iter().flatten().enumerate() {
            wire[index] = wire_scalar(value)?;
        }
        Ok(wire)
    }
}

fn checked_f64(value: f64) -> Result<f64, MathError> {
    value.is_finite().then_some(value).ok_or(MathError::Numeric)
}

fn wire_scalar(value: f64) -> Result<f32, MathError> {
    checked_f64(value)?;
    if value.abs() > f64::from(f32::MAX) {
        return Err(MathError::Numeric);
    }
    Ok(value as f32)
}

fn resolve_rows(rows: [[Result<f64, MathError>; 4]; 3]) -> Result<SnapshotAffine3x4, MathError> {
    Ok(SnapshotAffine3x4 {
        rows: [
            resolve_row(rows[0])?,
            resolve_row(rows[1])?,
            resolve_row(rows[2])?,
        ],
    })
}

fn resolve_row(row: [Result<f64, MathError>; 4]) -> Result<[f64; 4], MathError> {
    Ok([row[0]?, row[1]?, row[2]?, row[3]?])
}
