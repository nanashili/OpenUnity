//! Public acceptance for bounded F32 WAV offline playback.

use engine_audio::clip::SampleFormat;
use engine_audio::offline::{OfflineError, WavOfflineError, render_f32_wav_offline};
use engine_audio::wav::WavError;

struct Chunk {
    id: [u8; 4],
    payload: Vec<u8>,
}

fn wav(chunks: &[Chunk]) -> Vec<u8> {
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

fn fmt(tag: u16, channels: u16, rate: u32, bits: u16) -> Chunk {
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

fn f32_data(samples: &[f32]) -> Chunk {
    let mut payload = Vec::with_capacity(samples.len() * 4);
    for sample in samples {
        payload.extend_from_slice(&sample.to_bits().to_le_bytes());
    }
    Chunk {
        id: *b"data",
        payload,
    }
}

fn i16_data(samples: &[i16]) -> Chunk {
    let mut payload = Vec::with_capacity(samples.len() * 2);
    for sample in samples {
        payload.extend_from_slice(&sample.to_le_bytes());
    }
    Chunk {
        id: *b"data",
        payload,
    }
}

#[test]
fn renders_mono_f32_across_a_short_final_block() {
    let bytes = wav(&[fmt(3, 1, 48_000, 32), f32_data(&[0.25, -0.5, 1.0, -1.0])]);

    let output = render_f32_wav_offline(&bytes, 3).unwrap();
    assert_eq!(output.sample_rate_hz().get(), 48_000);
    assert_eq!(output.channels().get(), 1);
    assert_eq!(output.frame_count(), 4);
    assert_eq!(output.samples(), &[0.25, -0.5, 1.0, -1.0]);
}

#[test]
fn renders_stereo_f32_in_frame_major_order_with_one_frame_blocks() {
    let bytes = wav(&[fmt(3, 2, 22_050, 32), f32_data(&[0.25, -0.25, 0.5, -0.5])]);

    let output = render_f32_wav_offline(&bytes, 1).unwrap();
    assert_eq!(output.sample_rate_hz().get(), 22_050);
    assert_eq!(output.channels().get(), 2);
    assert_eq!(output.frame_count(), 2);
    assert_eq!(output.samples(), &[0.25, -0.25, 0.5, -0.5]);
}

#[test]
fn rejects_pcm16_without_converting_it() {
    let bytes = wav(&[fmt(1, 1, 8_000, 16), i16_data(&[1234])]);

    assert_eq!(
        render_f32_wav_offline(&bytes, 1),
        Err(WavOfflineError::UnsupportedSampleFormat(SampleFormat::I16))
    );
}

#[test]
fn preserves_the_importer_error_for_malformed_wave_bytes() {
    assert_eq!(
        render_f32_wav_offline(b"not a wave", 1),
        Err(WavOfflineError::Import(WavError::TruncatedRiffHeader))
    );
}

#[test]
fn zero_block_size_reaches_the_typed_renderer_error() {
    let bytes = wav(&[fmt(3, 1, 48_000, 32), f32_data(&[0.25])]);

    assert_eq!(
        render_f32_wav_offline(&bytes, 0),
        Err(WavOfflineError::Renderer(OfflineError::ZeroBlockSize))
    );
}

#[test]
fn canonicalizes_negative_zero_to_positive_zero_during_mixing() {
    let negative_zero = f32::from_bits(0x8000_0000);
    let bytes = wav(&[fmt(3, 1, 48_000, 32), f32_data(&[negative_zero])]);

    let output = render_f32_wav_offline(&bytes, 1).unwrap();

    assert_eq!(negative_zero, 0.0);
    assert_eq!(output.samples(), &[0.0]);
    assert_eq!(output.samples()[0].to_bits(), 0x0000_0000);
}
