//! Disposable independent affine calculations for preview-only markers.

use super::Affine;

impl Affine {
    pub(super) const IDENTITY: Self = Self([
        [1.0, 0.0, 0.0, 0.0],
        [0.0, 1.0, 0.0, 0.0],
        [0.0, 0.0, 1.0, 0.0],
    ]);

    pub(super) fn local(position: [f32; 3], quaternion: [f32; 4], scale: [f32; 3]) -> Self {
        let q = quaternion.map(f64::from);
        let norm = q.iter().map(|value| value * value).sum::<f64>().sqrt();
        let [x, y, z, w] = q.map(|value| value / norm);
        let mut matrix = [
            [
                1.0 - 2.0 * (y * y + z * z),
                2.0 * (x * y - z * w),
                2.0 * (x * z + y * w),
                f64::from(position[0]),
            ],
            [
                2.0 * (x * y + z * w),
                1.0 - 2.0 * (x * x + z * z),
                2.0 * (y * z - x * w),
                f64::from(position[1]),
            ],
            [
                2.0 * (x * z - y * w),
                2.0 * (y * z + x * w),
                1.0 - 2.0 * (x * x + y * y),
                f64::from(position[2]),
            ],
        ];
        for row in &mut matrix {
            for (value, scale) in row[..3].iter_mut().zip(scale) {
                *value *= f64::from(scale);
            }
        }
        Self(matrix)
    }

    pub(super) fn compose(self, local: Self) -> Option<Self> {
        let mut result = [[0.0; 4]; 3];
        for (i, row) in result.iter_mut().enumerate() {
            for (j, value) in row.iter_mut().enumerate() {
                *value = (0..3).map(|k| self.0[i][k] * local.0[k][j]).sum::<f64>();
                if j == 3 {
                    *value += self.0[i][3];
                }
            }
        }
        result
            .iter()
            .flatten()
            .all(|value| value.is_finite())
            .then_some(Self(result))
    }

    pub(super) fn apply(self, point: [f64; 3]) -> [f64; 3] {
        self.0
            .map(|row| row[0] * point[0] + row[1] * point[1] + row[2] * point[2] + row[3])
    }
}
