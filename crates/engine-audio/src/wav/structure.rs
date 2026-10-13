use super::error::WavError;

const MAX_INPUT_BYTES: usize = 16 * 1024 * 1024;
const MAX_CHUNKS: usize = 4_096;
const MAX_DATA_BYTES: usize = 8 * 1024 * 1024;
const MAX_FRAMES: usize = 1_048_576;
const MIN_SAMPLE_RATE: u32 = 8_000;
const MAX_SAMPLE_RATE: u32 = 192_000;

#[derive(Clone, Copy)]
pub(super) enum WavSamples {
    I16,
    F32,
}

#[derive(Clone, Copy)]
pub(super) struct WavFormat {
    pub(super) sample_rate_hz: u32,
    pub(super) channels: u16,
    pub(super) block_align: usize,
    pub(super) samples: WavSamples,
}

pub(super) struct WavLayout {
    pub(super) format: WavFormat,
    pub(super) data_start: usize,
    pub(super) data_len: usize,
}

pub(super) fn scan(bytes: &[u8]) -> Result<WavLayout, WavError> {
    validate_riff_header(bytes)?;
    let riff_end = checked_riff_end(bytes)?;
    let mut cursor = 12usize;
    let mut chunk_count = 0usize;
    let mut format = None;
    let mut data = None;

    while cursor < riff_end {
        chunk_count = chunk_count.checked_add(1).ok_or(WavError::SizeOverflow)?;
        if chunk_count > MAX_CHUNKS {
            return Err(WavError::ChunkCountExceeded);
        }
        let chunk = read_chunk(bytes, cursor, riff_end)?;
        if chunk.id == *b"fmt " {
            if format.is_some() {
                return Err(WavError::DuplicateFormat);
            }
            format = Some(parse_format(chunk.payload)?);
        } else if chunk.id == *b"data" {
            if format.is_none() {
                return Err(WavError::DataBeforeFormat);
            }
            if data.is_some() {
                return Err(WavError::DuplicateData);
            }
            data = Some((chunk.payload_start, chunk.payload.len()));
        }
        cursor = chunk.padded_end;
    }

    let format = format.ok_or(WavError::MissingFormat)?;
    let (data_start, data_len) = data.ok_or(WavError::MissingData)?;
    validate_data_bounds(format, data_len)?;
    Ok(WavLayout {
        format,
        data_start,
        data_len,
    })
}

fn validate_riff_header(bytes: &[u8]) -> Result<(), WavError> {
    if bytes.len() > MAX_INPUT_BYTES {
        return Err(WavError::InputTooLarge);
    }
    if bytes.len() < 12 {
        return Err(WavError::TruncatedRiffHeader);
    }
    if bytes.get(0..4) != Some(b"RIFF") || bytes.get(8..12) != Some(b"WAVE") {
        return Err(WavError::InvalidSignature);
    }
    Ok(())
}

fn checked_riff_end(bytes: &[u8]) -> Result<usize, WavError> {
    let riff_size = usize::try_from(read_u32(bytes, 4)).map_err(|_| WavError::SizeOverflow)?;
    if riff_size < 4 {
        return Err(WavError::RiffExtentTooSmall);
    }
    let riff_end = 8usize
        .checked_add(riff_size)
        .ok_or(WavError::SizeOverflow)?;
    if riff_end > bytes.len() {
        return Err(WavError::TruncatedRiff);
    }
    if riff_end < bytes.len() {
        return Err(WavError::TrailingInput);
    }
    Ok(riff_end)
}

struct Chunk<'a> {
    id: [u8; 4],
    payload_start: usize,
    payload: &'a [u8],
    padded_end: usize,
}

fn read_chunk<'a>(bytes: &'a [u8], cursor: usize, riff_end: usize) -> Result<Chunk<'a>, WavError> {
    let header_end = cursor.checked_add(8).ok_or(WavError::SizeOverflow)?;
    if header_end > riff_end {
        return Err(WavError::TruncatedChunkHeader);
    }
    let id_end = cursor.checked_add(4).ok_or(WavError::SizeOverflow)?;
    let size_offset = id_end;
    let id = bytes[cursor..id_end]
        .try_into()
        .map_err(|_| WavError::SizeOverflow)?;
    let payload_len =
        usize::try_from(read_u32(bytes, size_offset)).map_err(|_| WavError::SizeOverflow)?;
    let payload_end = header_end
        .checked_add(payload_len)
        .ok_or(WavError::SizeOverflow)?;
    if payload_end > riff_end {
        return Err(WavError::TruncatedChunkPayload);
    }
    let padded_end = payload_end
        .checked_add(payload_len & 1)
        .ok_or(WavError::SizeOverflow)?;
    if padded_end > riff_end {
        return Err(WavError::TruncatedChunkPad);
    }
    Ok(Chunk {
        id,
        payload_start: header_end,
        payload: &bytes[header_end..payload_end],
        padded_end,
    })
}

fn parse_format(payload: &[u8]) -> Result<WavFormat, WavError> {
    match payload.len() {
        16 => {}
        0..=15 | 17 => return Err(WavError::InvalidFormatLength),
        18 if read_u16(payload, 16) == 0 => {}
        18 => return Err(WavError::UnsupportedFormatExtension),
        _ => return Err(WavError::UnsupportedFormatExtension),
    }
    let tag = read_u16(payload, 0);
    let channels = read_u16(payload, 2);
    let sample_rate_hz = read_u32(payload, 4);
    let average_bytes_per_second = read_u32(payload, 8);
    let block_align = usize::from(read_u16(payload, 12));
    let bits = read_u16(payload, 14);

    let samples = match (tag, bits) {
        (1, 16) => WavSamples::I16,
        (3, 32) => WavSamples::F32,
        (1, _) | (3, _) => return Err(WavError::UnsupportedBitDepth(bits)),
        _ => return Err(WavError::UnsupportedFormatTag(tag)),
    };
    if !(1..=2).contains(&channels) {
        return Err(WavError::UnsupportedChannelCount(channels));
    }
    if !(MIN_SAMPLE_RATE..=MAX_SAMPLE_RATE).contains(&sample_rate_hz) {
        return Err(WavError::UnsupportedSampleRate(sample_rate_hz));
    }
    let bytes_per_sample = usize::from(bits / 8);
    let expected_block_align = usize::from(channels)
        .checked_mul(bytes_per_sample)
        .ok_or(WavError::SizeOverflow)?;
    if block_align != expected_block_align {
        return Err(WavError::InconsistentBlockAlignment);
    }
    let expected_byte_rate = u64::from(sample_rate_hz)
        .checked_mul(u64::try_from(block_align).map_err(|_| WavError::SizeOverflow)?)
        .ok_or(WavError::SizeOverflow)?;
    if expected_byte_rate != u64::from(average_bytes_per_second) {
        return Err(WavError::InconsistentByteRate);
    }
    Ok(WavFormat {
        sample_rate_hz,
        channels,
        block_align,
        samples,
    })
}

fn validate_data_bounds(format: WavFormat, data_len: usize) -> Result<(), WavError> {
    if data_len == 0 {
        return Err(WavError::EmptyData);
    }
    if data_len > MAX_DATA_BYTES {
        return Err(WavError::DataTooLarge);
    }
    if data_len % format.block_align != 0 {
        return Err(WavError::DataNotFrameAligned);
    }
    let frames = data_len / format.block_align;
    if frames > MAX_FRAMES {
        return Err(WavError::FrameLimitExceeded);
    }
    frames
        .checked_mul(usize::from(format.channels))
        .ok_or(WavError::SizeOverflow)?;
    Ok(())
}

fn read_u16(bytes: &[u8], offset: usize) -> u16 {
    u16::from_le_bytes([bytes[offset], bytes[offset + 1]])
}

fn read_u32(bytes: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes([
        bytes[offset],
        bytes[offset + 1],
        bytes[offset + 2],
        bytes[offset + 3],
    ])
}
