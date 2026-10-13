use engine_audio::wav::{WavError, import_wav};

#[derive(Clone)]
pub struct Chunk {
    pub id: [u8; 4],
    pub payload: Vec<u8>,
}

pub fn wav(chunks: &[Chunk]) -> Vec<u8> {
    let body_len = 4 + chunks.iter().map(encoded_len).sum::<usize>();
    let mut bytes = Vec::with_capacity(8 + body_len);
    bytes.extend_from_slice(b"RIFF");
    bytes.extend_from_slice(&(body_len as u32).to_le_bytes());
    bytes.extend_from_slice(b"WAVE");
    for chunk in chunks {
        bytes.extend_from_slice(&chunk.id);
        bytes.extend_from_slice(&(chunk.payload.len() as u32).to_le_bytes());
        bytes.extend_from_slice(&chunk.payload);
        if chunk.payload.len() % 2 == 1 {
            bytes.push(0);
        }
    }
    bytes
}

fn encoded_len(chunk: &Chunk) -> usize {
    8 + chunk.payload.len() + chunk.payload.len() % 2
}

pub fn fmt_chunk(tag: u16, channels: u16, rate: u32, bits: u16) -> Chunk {
    let block_align = channels * (bits / 8);
    let byte_rate = rate * u32::from(block_align);
    let mut payload = Vec::with_capacity(16);
    payload.extend_from_slice(&tag.to_le_bytes());
    payload.extend_from_slice(&channels.to_le_bytes());
    payload.extend_from_slice(&rate.to_le_bytes());
    payload.extend_from_slice(&byte_rate.to_le_bytes());
    payload.extend_from_slice(&block_align.to_le_bytes());
    payload.extend_from_slice(&bits.to_le_bytes());
    Chunk {
        id: *b"fmt ",
        payload,
    }
}

pub fn fmt_extension(cb_size: u16) -> Chunk {
    let mut chunk = fmt_chunk(1, 1, 48_000, 16);
    chunk.payload.extend_from_slice(&cb_size.to_le_bytes());
    chunk
}

pub fn pcm16(samples: &[i16]) -> Chunk {
    let mut payload = Vec::with_capacity(samples.len() * 2);
    for sample in samples {
        payload.extend_from_slice(&sample.to_le_bytes());
    }
    Chunk {
        id: *b"data",
        payload,
    }
}

pub fn float32(samples: &[f32]) -> Chunk {
    let mut payload = Vec::with_capacity(samples.len() * 4);
    for sample in samples {
        payload.extend_from_slice(&sample.to_bits().to_le_bytes());
    }
    Chunk {
        id: *b"data",
        payload,
    }
}

pub fn unknown(id: &[u8; 4], payload: Vec<u8>) -> Chunk {
    Chunk { id: *id, payload }
}

pub fn base() -> Vec<u8> {
    wav(&[fmt_chunk(1, 1, 48_000, 16), pcm16(&[1])])
}

pub fn expect_error(bytes: &[u8], expected: WavError) {
    assert_eq!(import_wav(bytes), Err(expected));
}

pub fn truncate_declared_tail(mut bytes: Vec<u8>, removed: usize) -> Vec<u8> {
    let riff_size = u32::from_le_bytes(bytes[4..8].try_into().unwrap());
    bytes.truncate(bytes.len() - removed);
    bytes[4..8].copy_from_slice(&(riff_size - removed as u32).to_le_bytes());
    bytes
}
