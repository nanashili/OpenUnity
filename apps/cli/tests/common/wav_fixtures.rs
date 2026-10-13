use std::path::PathBuf;

use super::workspace::Workspace;

pub(crate) struct Chunk {
    pub(crate) id: [u8; 4],
    pub(crate) payload: Vec<u8>,
}

pub(crate) fn wav(chunks: &[Chunk]) -> Vec<u8> {
    let riff_size = 4 + chunks.iter().map(encoded_len).sum::<usize>();
    let mut bytes = Vec::with_capacity(8 + riff_size);
    bytes.extend_from_slice(b"RIFF");
    bytes.extend_from_slice(&(riff_size as u32).to_le_bytes());
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

pub(crate) fn fmt_chunk(channels: u16, rate: u32) -> Chunk {
    let block_align = channels * 4;
    let mut payload = Vec::with_capacity(16);
    payload.extend_from_slice(&3u16.to_le_bytes());
    payload.extend_from_slice(&channels.to_le_bytes());
    payload.extend_from_slice(&rate.to_le_bytes());
    payload.extend_from_slice(&(rate * u32::from(block_align)).to_le_bytes());
    payload.extend_from_slice(&block_align.to_le_bytes());
    payload.extend_from_slice(&32u16.to_le_bytes());

    Chunk {
        id: *b"fmt ",
        payload,
    }
}

pub(crate) fn f32_data(samples: &[f32]) -> Chunk {
    let mut payload = Vec::with_capacity(samples.len() * 4);

    for sample in samples {
        payload.extend_from_slice(&sample.to_bits().to_le_bytes());
    }

    Chunk {
        id: *b"data",
        payload,
    }
}

pub(crate) fn mono(workspace: &Workspace, name: &str, samples: &[f32]) -> PathBuf {
    workspace.write(name, &wav(&[fmt_chunk(1, 48_000), f32_data(samples)]))
}
