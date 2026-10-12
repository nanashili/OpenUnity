use crate::HostError;
use crate::abi::{SNAPSHOT_ROW_STRIDE, SNAPSHOT_VERSION};

const HEADER_SIZE: usize = 16;
const ROW_SIZE: usize = SNAPSHOT_ROW_STRIDE as usize;
const MAX_ROWS: usize = 4097;

/// Immutable values decoded from one native scene snapshot payload.
#[derive(Clone, Debug, PartialEq)]
pub struct SceneSnapshot {
    frame: i32,
    rows: Vec<SceneSnapshotRow>,
}

impl SceneSnapshot {
    /// Returns the captured native scene frame number.
    pub fn frame(&self) -> i32 {
        self.frame
    }

    /// Returns snapshot rows in the order of the requested local file IDs.
    pub fn rows(&self) -> &[SceneSnapshotRow] {
        &self.rows
    }
}

/// The captured lifecycle state of a requested scene object.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SnapshotState {
    /// The requested object was absent from the captured scene.
    Dead,
    /// The object exists but is inactive.
    AliveInactive,
    /// The object exists and is active.
    AliveActive,
}

/// Owned scene data decoded for one requested local file ID.
#[derive(Clone, Debug, PartialEq)]
pub struct SceneSnapshotRow {
    local_file_id: i64,
    state: SnapshotState,
    unit_scale_chain: bool,
    affine: [f32; 12],
}

impl SceneSnapshotRow {
    /// Returns the local file ID supplied for this row.
    pub fn local_file_id(&self) -> i64 {
        self.local_file_id
    }

    /// Returns the object's captured lifecycle state.
    pub fn state(&self) -> SnapshotState {
        self.state
    }

    /// Reports whether the owner and every current ancestor have local scale exactly `(1, 1, 1)`.
    pub fn unit_scale_chain(&self) -> bool {
        self.unit_scale_chain
    }

    /// Returns the row's 12 affine coefficients.
    pub fn affine(&self) -> [f32; 12] {
        self.affine
    }
}

pub(super) fn required_size(row_count: usize) -> Result<usize, HostError> {
    if !(1..=MAX_ROWS).contains(&row_count) {
        return Err(HostError::status("capture scene snapshot", -2));
    }
    HEADER_SIZE
        .checked_add(row_count.checked_mul(ROW_SIZE).ok_or_else(malformed)?)
        .ok_or_else(malformed)
}

pub(super) fn decode(payload: &[u8], requested_ids: &[i64]) -> Result<SceneSnapshot, HostError> {
    let required = required_size(requested_ids.len())?;
    if payload.len() != required {
        return Err(malformed());
    }
    if read_u32(payload, 0) != SNAPSHOT_VERSION
        || read_i32(payload, 8) != requested_ids.len() as i32
        || read_u32(payload, 12) != SNAPSHOT_ROW_STRIDE
        || read_i32(payload, 4) < 0
    {
        return Err(malformed());
    }

    let mut rows = Vec::with_capacity(requested_ids.len());
    for (index, requested_id) in requested_ids.iter().copied().enumerate() {
        rows.push(decode_row(payload, index, requested_id)?);
    }
    Ok(SceneSnapshot {
        frame: read_i32(payload, 4),
        rows,
    })
}

fn decode_row(
    payload: &[u8],
    index: usize,
    requested_id: i64,
) -> Result<SceneSnapshotRow, HostError> {
    let offset = HEADER_SIZE + index * ROW_SIZE;
    if read_i64(payload, offset) != requested_id {
        return Err(malformed());
    }
    let state = match read_i32(payload, offset + 8) {
        0 => SnapshotState::Dead,
        1 => SnapshotState::AliveInactive,
        2 => SnapshotState::AliveActive,
        _ => return Err(malformed()),
    };
    let unit_scale_chain = match read_i32(payload, offset + 12) {
        0 => false,
        1 => true,
        _ => return Err(malformed()),
    };
    if state == SnapshotState::Dead && unit_scale_chain {
        return Err(malformed());
    }

    let mut affine = [0.0_f32; 12];
    for (index, value) in affine.iter_mut().enumerate() {
        let bits = read_u32(payload, offset + 16 + index * 4);
        if state == SnapshotState::Dead {
            if bits != 0 {
                return Err(malformed());
            }
        } else {
            *value = f32::from_bits(bits);
            if !value.is_finite() {
                return Err(malformed());
            }
        }
    }
    Ok(SceneSnapshotRow {
        local_file_id: requested_id,
        state,
        unit_scale_chain,
        affine,
    })
}

fn malformed() -> HostError {
    HostError::status("capture scene snapshot", -6)
}

fn read_u32(payload: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes(payload[offset..offset + 4].try_into().unwrap())
}

fn read_i32(payload: &[u8], offset: usize) -> i32 {
    i32::from_le_bytes(payload[offset..offset + 4].try_into().unwrap())
}

fn read_i64(payload: &[u8], offset: usize) -> i64 {
    i64::from_le_bytes(payload[offset..offset + 8].try_into().unwrap())
}

#[cfg(test)]
#[path = "snapshot_tests.rs"]
mod tests;
