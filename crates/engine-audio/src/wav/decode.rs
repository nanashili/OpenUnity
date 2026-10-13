use crate::clip::PcmClip;

use super::error::WavError;
use super::structure::{WavLayout, WavSamples};

pub(super) fn decode_clip(bytes: &[u8], layout: WavLayout) -> Result<PcmClip, WavError> {
    let sample_count = layout
        .data_len
        .checked_div(layout.format.block_align)
        .and_then(|frames| frames.checked_mul(usize::from(layout.format.channels)))
        .ok_or(WavError::SizeOverflow)?;
    match layout.format.samples {
        WavSamples::I16 => decode_i16(bytes, layout, sample_count),
        WavSamples::F32 => decode_f32(bytes, layout, sample_count),
    }
}

fn decode_i16(bytes: &[u8], layout: WavLayout, sample_count: usize) -> Result<PcmClip, WavError> {
    let mut samples = Vec::new();
    samples
        .try_reserve_exact(sample_count)
        .map_err(|_| WavError::SampleAllocationFailed)?;
    for index in 0..sample_count {
        let offset = sample_offset(layout.data_start, index, 2)?;
        samples.push(read_i16(bytes, offset)?);
    }
    PcmClip::from_i16(
        layout.format.sample_rate_hz,
        layout.format.channels,
        &samples,
    )
    .map_err(WavError::PcmClip)
}

fn decode_f32(bytes: &[u8], layout: WavLayout, sample_count: usize) -> Result<PcmClip, WavError> {
    let mut samples = Vec::new();
    samples
        .try_reserve_exact(sample_count)
        .map_err(|_| WavError::SampleAllocationFailed)?;
    for index in 0..sample_count {
        let offset = sample_offset(layout.data_start, index, 4)?;
        let bits = read_u32(bytes, offset)?;
        samples.push(f32::from_bits(bits));
    }
    PcmClip::from_f32(
        layout.format.sample_rate_hz,
        layout.format.channels,
        &samples,
    )
    .map_err(WavError::PcmClip)
}

fn sample_offset(data_start: usize, index: usize, width: usize) -> Result<usize, WavError> {
    index
        .checked_mul(width)
        .and_then(|offset| data_start.checked_add(offset))
        .ok_or(WavError::SizeOverflow)
}

fn read_i16(bytes: &[u8], offset: usize) -> Result<i16, WavError> {
    let end = offset.checked_add(2).ok_or(WavError::SizeOverflow)?;
    let raw = bytes
        .get(offset..end)
        .ok_or(WavError::TruncatedChunkPayload)?;
    Ok(i16::from_le_bytes(
        raw.try_into().map_err(|_| WavError::SizeOverflow)?,
    ))
}

fn read_u32(bytes: &[u8], offset: usize) -> Result<u32, WavError> {
    let end = offset.checked_add(4).ok_or(WavError::SizeOverflow)?;
    let raw = bytes
        .get(offset..end)
        .ok_or(WavError::TruncatedChunkPayload)?;
    Ok(u32::from_le_bytes(
        raw.try_into().map_err(|_| WavError::SizeOverflow)?,
    ))
}
