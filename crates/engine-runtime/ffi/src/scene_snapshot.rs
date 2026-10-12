//! Checked native byte transport for same-session immutable scene snapshots.

use crate::runtime_store::with_runtime_scene_snapshot;
use crate::status::{INSUFFICIENT_CAPACITY, INVALID_BUFFER, runtime_error};
use engine_runtime::{Runtime, SNAPSHOT_ROW_STRIDE, SNAPSHOT_VERSION};

/// ABI version returned by the scene snapshot entry point.
pub const SCENE_SNAPSHOT_ABI_VERSION: u32 = SNAPSHOT_VERSION;
const HEADER_BYTES: usize = 16;
const ROW_STRIDE_BYTES: usize = SNAPSHOT_ROW_STRIDE as usize;

#[unsafe(no_mangle)]
/// Return the scene snapshot ABI version supported by this library.
pub extern "C" fn ou_scene_snapshot_abi_version() -> u32 {
    SCENE_SNAPSHOT_ABI_VERSION
}

fn checked_range(pointer: *const u8, bytes: usize) -> Result<(usize, usize), i32> {
    if pointer.is_null() {
        return Err(INVALID_BUFFER);
    }
    let start = pointer as usize;
    let end = start.checked_add(bytes).ok_or(INVALID_BUFFER)?;
    Ok((start, end))
}

fn validate_buffers(
    owner_file_ids: *const i64,
    count: i32,
    output: *mut u8,
    capacity: i32,
) -> Result<usize, i32> {
    let count = usize::try_from(count)
        .ok()
        .filter(|count| (1..=4097).contains(count))
        .ok_or(INVALID_BUFFER)?;
    let capacity = usize::try_from(capacity).map_err(|_| INVALID_BUFFER)?;
    let alignment = std::mem::align_of::<i64>();
    if (owner_file_ids as usize) % alignment != 0 {
        return Err(INVALID_BUFFER);
    }
    let input_bytes = count
        .checked_mul(std::mem::size_of::<i64>())
        .ok_or(INVALID_BUFFER)?;
    let required = required_bytes(count)?;
    let input_range = checked_range(owner_file_ids.cast(), input_bytes)?;
    let output_range = checked_range(output.cast_const(), capacity)?;
    if input_range.0 < output_range.1 && output_range.0 < input_range.1 {
        return Err(INVALID_BUFFER);
    }
    if capacity < required {
        return Err(INSUFFICIENT_CAPACITY);
    }
    Ok(count)
}

fn required_bytes(count: usize) -> Result<usize, i32> {
    count
        .checked_mul(ROW_STRIDE_BYTES)
        .and_then(|rows| HEADER_BYTES.checked_add(rows))
        .ok_or(INVALID_BUFFER)
}

fn row_offset(index: usize) -> Result<usize, i32> {
    index
        .checked_mul(ROW_STRIDE_BYTES)
        .and_then(|row| HEADER_BYTES.checked_add(row))
        .ok_or(crate::status::COUNT_OVERFLOW)
}

fn write_u32(output: &mut [u8], offset: usize, value: u32) {
    output[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
}

fn write_i32(output: &mut [u8], offset: usize, value: i32) {
    output[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
}

fn write_i64(output: &mut [u8], offset: usize, value: i64) {
    output[offset..offset + 8].copy_from_slice(&value.to_le_bytes());
}

fn serialize(snapshot: &engine_runtime::SceneSnapshot, output: &mut [u8]) -> Result<(), i32> {
    write_u32(output, 0, SNAPSHOT_VERSION);
    write_i32(output, 4, snapshot.frame());
    let row_count =
        i32::try_from(snapshot.row_count()).map_err(|_| crate::status::COUNT_OVERFLOW)?;
    write_i32(output, 8, row_count);
    write_u32(output, 12, SNAPSHOT_ROW_STRIDE);
    for index in 0..snapshot.row_count() {
        let row = snapshot.row(index).ok_or(crate::status::COUNT_OVERFLOW)?;
        let offset = row_offset(index)?;
        write_i64(output, offset, row.local_file_id());
        let state = match row.state() {
            engine_runtime::SnapshotState::Dead => 0,
            engine_runtime::SnapshotState::AliveInactive => 1,
            engine_runtime::SnapshotState::AliveActive => 2,
        };
        write_i32(output, offset + 8, state);
        write_i32(output, offset + 12, i32::from(row.unit_scale_chain()));
        for (coefficient, value) in row.affine().iter().enumerate() {
            let coefficient_offset = offset
                .checked_add(16)
                .and_then(|base| {
                    coefficient
                        .checked_mul(4)
                        .and_then(|delta| base.checked_add(delta))
                })
                .ok_or(crate::status::COUNT_OVERFLOW)?;
            let coefficient_end = coefficient_offset
                .checked_add(4)
                .ok_or(crate::status::COUNT_OVERFLOW)?;
            output[coefficient_offset..coefficient_end].copy_from_slice(&value.to_le_bytes());
        }
    }
    Ok(())
}

#[unsafe(no_mangle)]
/// Capture scene owners into a caller-provided binary snapshot buffer.
///
/// The output begins with a fixed 16-byte header followed by 64-byte rows.
/// Negative return values identify invalid input or runtime failures.
///
/// # Safety
/// `owner_file_ids` must be readable and naturally aligned for `count` `i64`
/// values, and `output` must be writable for `capacity` bytes. The two memory
/// regions must not overlap. Pointers must remain valid for this synchronous
/// call; the function does not retain either pointer after returning.
pub unsafe extern "C" fn ou_scene_snapshot(
    runtime: i64,
    scene: i64,
    owner_file_ids: *const i64,
    count: i32,
    output: *mut u8,
    capacity: i32,
) -> i32 {
    let count = match validate_buffers(owner_file_ids, count, output, capacity) {
        Ok(count) => count,
        Err(error) => return error,
    };
    // SAFETY: the caller owns a readable, naturally aligned region for count i64s
    // for the duration of this synchronous call; the copy does not retain a pointer.
    let ids = unsafe { std::slice::from_raw_parts(owner_file_ids, count).to_vec() };
    let result = with_runtime_scene_snapshot(runtime, scene, |runtime: &mut Runtime, scene_id| {
        runtime.scene_snapshot(scene_id, &ids)
    });
    let snapshot = match result {
        Ok(Ok(snapshot)) => snapshot,
        Ok(Err(error)) => return runtime_error(error),
        Err(error) => return error,
    };
    let required = match required_bytes(count) {
        Ok(required) => required,
        Err(error) => return error,
    };
    let mut encoded = vec![0_u8; required];
    if let Err(error) = serialize(&snapshot, &mut encoded) {
        return error;
    }
    // SAFETY: the caller guarantees writable storage for capacity bytes.
    // Validation checked range arithmetic, capacity, and non-overlap with the input.
    unsafe {
        std::ptr::copy_nonoverlapping(encoded.as_ptr(), output, required);
    }
    0
}

#[cfg(test)]
#[path = "scene_snapshot/tests.rs"]
mod tests;
