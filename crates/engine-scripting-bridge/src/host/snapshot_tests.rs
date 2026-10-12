use super::{SceneSnapshot, SnapshotState, decode, required_size};
use crate::abi::{SNAPSHOT_ROW_STRIDE, SNAPSHOT_VERSION};

const HEADER_SIZE: usize = 16;
const ROW_SIZE: usize = SNAPSHOT_ROW_STRIDE as usize;

#[test]
fn wire_constants_match_literal_contract() {
    assert_eq!(SNAPSHOT_VERSION, 0x0001_0000);
    assert_eq!(SNAPSHOT_ROW_STRIDE, 64);
    assert_eq!(HEADER_SIZE, 16);
    assert_eq!(ROW_SIZE, 64);
}

#[test]
fn decodes_ordered_dead_and_live_rows() {
    let ids = [17, 29];
    let mut payload = valid_payload(&ids);
    let live_row_offset = HEADER_SIZE + ROW_SIZE;
    write_i32(&mut payload, live_row_offset + 8, 2);
    write_i32(&mut payload, live_row_offset + 12, 1);
    write_u32(&mut payload, live_row_offset + 16, 1.5_f32.to_bits());

    let snapshot = decode(&payload, &ids).expect("valid snapshot");

    assert_eq!(snapshot.frame(), 43);
    assert_eq!(snapshot.rows()[0].local_file_id(), 17);
    assert_eq!(snapshot.rows()[0].state(), SnapshotState::Dead);
    assert!(!snapshot.rows()[0].unit_scale_chain());
    assert_eq!(snapshot.rows()[0].affine(), [0.0; 12]);
    assert_eq!(snapshot.rows()[1].local_file_id(), 29);
    assert_eq!(snapshot.rows()[1].state(), SnapshotState::AliveActive);
    assert!(snapshot.rows()[1].unit_scale_chain());
    assert_eq!(snapshot.rows()[1].affine()[0], 1.5);
}

#[test]
fn rejects_wrong_header_shape_and_requested_identity() {
    let ids = [17, 29];
    let payload = valid_payload(&ids);

    assert_status(decode(&payload[..payload.len() - 1], &ids), -6);

    let mut wrong_version = payload.clone();
    write_u32(&mut wrong_version, 0, SNAPSHOT_VERSION + 1);
    assert_status(decode(&wrong_version, &ids), -6);

    let mut wrong_count = payload.clone();
    write_i32(&mut wrong_count, 8, 3);
    assert_status(decode(&wrong_count, &ids), -6);

    let mut wrong_stride = payload.clone();
    write_u32(&mut wrong_stride, 12, SNAPSHOT_ROW_STRIDE + 1);
    assert_status(decode(&wrong_stride, &ids), -6);

    let mut trailing_byte = payload.clone();
    trailing_byte.push(0);
    assert_status(decode(&trailing_byte, &ids), -6);

    let mut wrong_id = payload;
    write_i64(&mut wrong_id, HEADER_SIZE, 18);
    assert_status(decode(&wrong_id, &ids), -6);

    let mut reversed_rows = valid_payload(&ids);
    write_i64(&mut reversed_rows, HEADER_SIZE, 29);
    write_i64(&mut reversed_rows, HEADER_SIZE + ROW_SIZE, 17);
    assert_status(decode(&reversed_rows, &ids), -6);
}

#[test]
fn accepts_frame_zero_and_rejects_negative_frame() {
    let ids = [17];
    let mut frame_zero = valid_payload(&ids);
    write_i32(&mut frame_zero, 4, 0);

    assert_eq!(decode(&frame_zero, &ids).expect("frame zero").frame(), 0);

    let mut negative_frame = valid_payload(&ids);
    write_i32(&mut negative_frame, 4, -1);

    assert_status(decode(&negative_frame, &ids), -6);
}

#[test]
fn rejects_invalid_row_state_and_flags() {
    let ids = [17];
    let mut invalid_state = valid_payload(&ids);
    write_i32(&mut invalid_state, HEADER_SIZE + 8, 3);
    assert_status(decode(&invalid_state, &ids), -6);

    let mut invalid_flag = valid_payload(&ids);
    write_i32(&mut invalid_flag, HEADER_SIZE + 12, 2);
    assert_status(decode(&invalid_flag, &ids), -6);
}

#[test]
fn rejects_noncanonical_dead_rows() {
    let ids = [17];
    let mut dead_with_chain = valid_payload(&ids);
    write_i32(&mut dead_with_chain, HEADER_SIZE + 12, 1);
    assert_status(decode(&dead_with_chain, &ids), -6);

    let mut dead_negative_zero = valid_payload(&ids);
    write_u32(
        &mut dead_negative_zero,
        HEADER_SIZE + 16,
        (-0.0_f32).to_bits(),
    );
    assert_status(decode(&dead_negative_zero, &ids), -6);
}

#[test]
fn rejects_nonfinite_live_coefficients() {
    let ids = [17];
    for nonfinite in [f32::INFINITY, f32::NEG_INFINITY, f32::NAN] {
        let mut payload = valid_payload(&ids);
        write_i32(&mut payload, HEADER_SIZE + 8, 1);
        write_u32(&mut payload, HEADER_SIZE + 16, nonfinite.to_bits());

        assert_status(decode(&payload, &ids), -6);
    }
}

#[test]
fn rejects_unsupported_row_counts() {
    assert!(required_size(0).is_err());
    assert!(required_size(4098).is_err());
    assert_status(decode(&[], &[]), -2);

    let maximum_ids: Vec<i64> = (1..=4097).map(i64::from).collect();
    let maximum_payload = valid_payload(&maximum_ids);
    let snapshot = decode(&maximum_payload, &maximum_ids).expect("maximum row count");

    assert_eq!(snapshot.rows().len(), 4097);
}

fn valid_payload(ids: &[i64]) -> Vec<u8> {
    let mut payload = vec![0; required_size(ids.len()).expect("bounded row count")];
    write_u32(&mut payload, 0, SNAPSHOT_VERSION);
    write_i32(&mut payload, 4, 43);
    write_i32(&mut payload, 8, ids.len() as i32);
    write_u32(&mut payload, 12, SNAPSHOT_ROW_STRIDE);

    for (index, id) in ids.iter().copied().enumerate() {
        write_i64(&mut payload, HEADER_SIZE + index * ROW_SIZE, id);
    }
    payload
}

fn assert_status(result: Result<SceneSnapshot, crate::HostError>, expected: i32) {
    assert_eq!(result.expect_err("invalid snapshot").code(), Some(expected));
}

fn write_u32(payload: &mut [u8], offset: usize, value: u32) {
    payload[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
}

fn write_i32(payload: &mut [u8], offset: usize, value: i32) {
    payload[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
}

fn write_i64(payload: &mut [u8], offset: usize, value: i64) {
    payload[offset..offset + 8].copy_from_slice(&value.to_le_bytes());
}
