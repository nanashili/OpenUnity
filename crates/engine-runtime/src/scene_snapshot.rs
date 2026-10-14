//! Immutable, value-only scene snapshots assembled while the runtime store is locked.

use crate::{Runtime, RuntimeError};
use engine_math::SnapshotAffine3x4;
use engine_object::ObjectId;
use std::collections::BTreeSet;

/// Version of the immutable scene snapshot wire contract.
pub const SNAPSHOT_VERSION: u32 = 0x0001_0000;
/// Number of bytes reserved for each serialized snapshot row.
pub const SNAPSHOT_ROW_STRIDE: u32 = 64;
/// Maximum number of owner rows accepted by one snapshot request.
pub const SNAPSHOT_MAX_ROWS: usize = 4097;

/// Immutable, value-owned scene state captured from one runtime frame.
#[derive(Clone, Debug, PartialEq)]
pub struct SceneSnapshot {
    frame: i32,
    rows: Vec<SceneSnapshotRow>,
}

/// Value-owned state for one requested scene owner.
#[derive(Clone, Debug, PartialEq)]
pub struct SceneSnapshotRow {
    local_file_id: i64,
    state: SnapshotState,
    unit_scale_chain: bool,
    affine: [f32; 12],
}

/// Runtime state encoded for a scene snapshot row.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SnapshotState {
    /// The requested owner is a known, retired object.
    Dead,
    /// The requested owner exists but is inactive in the hierarchy.
    AliveInactive,
    /// The requested owner exists and is active in the hierarchy.
    AliveActive,
}

impl SceneSnapshot {
    /// Returns the runtime frame at which this snapshot was captured.
    pub fn frame(&self) -> i32 {
        self.frame
    }

    /// Returns the number of rows captured in this snapshot.
    pub fn row_count(&self) -> usize {
        self.rows.len()
    }

    /// Returns a row by zero-based index.
    pub fn row(&self, index: usize) -> Option<&SceneSnapshotRow> {
        self.rows.get(index)
    }
}

impl SceneSnapshotRow {
    /// Returns the serialized local file identifier for this owner.
    pub fn local_file_id(&self) -> i64 {
        self.local_file_id
    }

    /// Returns the owner lifetime and activation state.
    pub fn state(&self) -> SnapshotState {
        self.state
    }

    /// Reports whether every transform in the owner chain has unit scale.
    pub fn unit_scale_chain(&self) -> bool {
        self.unit_scale_chain
    }

    /// Returns the row's affine transform in row-major 3x4 form.
    pub fn affine(&self) -> &[f32; 12] {
        &self.affine
    }
}

impl Runtime {
    /// Capture all requested owners from one coherent runtime critical section.
    ///
    /// The returned values own every coefficient. No runtime borrow survives this
    /// call, so callers may retain a snapshot across ticks and scene release.
    pub fn scene_snapshot(
        &self,
        scene: ObjectId,
        local_file_ids: &[i64],
    ) -> Result<SceneSnapshot, RuntimeError> {
        if !(1..=SNAPSHOT_MAX_ROWS).contains(&local_file_ids.len()) {
            return Err(RuntimeError::CountOverflow);
        }
        self.scene_snapshot_admission(scene)?;

        let mut seen = BTreeSet::new();
        let mut rows = Vec::with_capacity(local_file_ids.len());
        for &local_file_id in local_file_ids {
            if local_file_id <= 0 {
                return Err(RuntimeError::InvalidObject);
            }
            if !seen.insert(local_file_id) {
                return Err(RuntimeError::InvalidObject);
            }
            let (owner, dead) = self.scene_snapshot_target(scene, local_file_id)?;
            if dead {
                rows.push(SceneSnapshotRow::dead(local_file_id));
                continue;
            }
            let owner = owner.ok_or(RuntimeError::InvalidObject)?;
            let affine = self.scene_snapshot_affine(owner)?;
            let unit_scale_chain = self.scene_snapshot_unit_scale(owner)?;
            let state = if self.gameobject_active(owner, true)? {
                SnapshotState::AliveActive
            } else {
                SnapshotState::AliveInactive
            };
            rows.push(SceneSnapshotRow {
                local_file_id,
                state,
                unit_scale_chain,
                affine,
            });
        }

        Ok(SceneSnapshot {
            frame: self.lifecycle_frame_count(),
            rows,
        })
    }

    fn scene_snapshot_admission(&self, scene: ObjectId) -> Result<(), RuntimeError> {
        self.scene_snapshot_scene_ready(scene)?;
        self.scene_snapshot_lifecycle_ready()
    }

    fn scene_snapshot_affine(&self, owner: ObjectId) -> Result<[f32; 12], RuntimeError> {
        let mut chain = Vec::new();
        let mut current = Some(self.gameobject_transform(owner)?);
        while let Some(transform) = current {
            chain.push(transform);
            current = self.transform_parent_id(transform)?;
        }

        let mut matrix = SnapshotAffine3x4::identity();
        for transform in chain.into_iter().rev() {
            let data = self.transform(transform)?;
            let local = SnapshotAffine3x4::from_trs(data.position, data.rotation, data.scale)?;
            matrix = matrix.compose(&local)?;
        }
        Ok(matrix.to_wire_rows()?)
    }

    fn scene_snapshot_unit_scale(&self, owner: ObjectId) -> Result<bool, RuntimeError> {
        let mut current = Some(self.gameobject_transform(owner)?);
        while let Some(transform) = current {
            let data = self.transform(transform)?;
            if data.scale != [1.0; 3] {
                return Ok(false);
            }
            current = self.transform_parent_id(transform)?;
        }
        Ok(true)
    }
}

impl SceneSnapshotRow {
    fn dead(local_file_id: i64) -> Self {
        Self {
            local_file_id,
            state: SnapshotState::Dead,
            unit_scale_chain: false,
            affine: [0.0; 12],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{SceneSnapshotRow, SnapshotState};

    #[test]
    fn dead_rows_have_positive_zero_pose() {
        let row = SceneSnapshotRow::dead(200);
        assert_eq!(row.state(), SnapshotState::Dead);
        assert!(!row.unit_scale_chain());
        assert!(row.affine().iter().all(|value| value.to_bits() == 0));
    }
}
