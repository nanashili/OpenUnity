use super::common::{float32, fmt_chunk, fmt_extension, pcm16, unknown, wav};
use engine_audio::wav::import_wav;

#[test]
fn imports_pcm16_mono_and_stereo_without_conversion() {
    let mono = wav(&[
        fmt_chunk(1, 1, 48_000, 16),
        pcm16(&[0, 1, 32_767, -32_768, -1, 4_660, -4_660]),
    ]);
    let clip = import_wav(&mono).unwrap();
    assert_eq!(clip.sample_rate_hz(), 48_000);
    assert_eq!(clip.channel_count(), 1);
    assert_eq!(clip.frame_count(), 7);
    assert_eq!(
        clip.i16_samples(),
        Some(&[0, 1, 32_767, -32_768, -1, 4_660, -4_660][..])
    );

    let stereo = wav(&[
        fmt_chunk(1, 2, 44_100, 16),
        pcm16(&[-32_768, 0, 32_767, 16_384]),
    ]);
    let clip = import_wav(&stereo).unwrap();
    assert_eq!(clip.channel_count(), 2);
    assert_eq!(clip.frame_count(), 2);
    assert_eq!(clip.i16_sample(0, 0), Some(-32_768));
    assert_eq!(clip.i16_sample(0, 1), Some(0));
    assert_eq!(clip.i16_sample(1, 0), Some(32_767));
    assert_eq!(clip.i16_sample(1, 1), Some(16_384));
}

#[test]
fn imports_float32_mono_and_stereo_with_public_pcm_validation() {
    let mono = wav(&[fmt_chunk(3, 1, 48_000, 32), float32(&[-1.0, 0.0, 0.5, 1.0])]);
    let clip = import_wav(&mono).unwrap();
    assert_eq!(clip.sample_format(), engine_audio::clip::SampleFormat::F32);
    assert_eq!(clip.f32_samples(), Some(&[-1.0, 0.0, 0.5, 1.0][..]));

    let stereo = wav(&[
        fmt_chunk(3, 2, 96_000, 32),
        float32(&[-1.0, 1.0, 0.25, -0.5]),
    ]);
    let clip = import_wav(&stereo).unwrap();
    assert_eq!(clip.channel_count(), 2);
    assert_eq!(clip.frame_count(), 2);
    assert_eq!(clip.f32_sample(1, 1), Some(-0.5));
}

#[test]
fn skips_unknown_chunks_and_consumes_odd_padding() {
    let mut bytes = wav(&[
        fmt_chunk(1, 1, 48_000, 16),
        unknown(b"JUNK", vec![1, 2, 3]),
        pcm16(&[100, -100]),
        unknown(b"LIST", vec![9, 8]),
    ]);

    bytes[47] = 0xA5;
    let clip = import_wav(&bytes).unwrap();
    assert_eq!(clip.i16_samples(), Some(&[100, -100][..]));
}

#[test]
fn skips_even_unknown_chunk_payload() {
    let bytes = wav(&[
        fmt_chunk(1, 1, 48_000, 16),
        unknown(b"JUNK", vec![1, 2, 3, 4]),
        pcm16(&[4660, -4660]),
    ]);
    let clip = import_wav(&bytes).unwrap();
    assert_eq!(clip.i16_samples(), Some(&[4660, -4660][..]));
}

#[test]
fn accepts_zero_extension_size_and_rate_boundaries() {
    let bytes = wav(&[fmt_extension(0), pcm16(&[7])]);
    let clip = import_wav(&bytes).unwrap();
    assert_eq!(clip.sample_rate_hz(), 48_000);
    assert_eq!(clip.i16_sample(0, 0), Some(7));

    let mut float_extension = fmt_extension(0);
    float_extension.payload[0..2].copy_from_slice(&3u16.to_le_bytes());
    float_extension.payload[4..8].copy_from_slice(&192_000u32.to_le_bytes());
    float_extension.payload[8..12].copy_from_slice(&768_000u32.to_le_bytes());
    float_extension.payload[12..14].copy_from_slice(&4u16.to_le_bytes());
    float_extension.payload[14..16].copy_from_slice(&32u16.to_le_bytes());
    let float_bytes = wav(&[float_extension, float32(&[0.25])]);
    let float_clip = import_wav(&float_bytes).unwrap();
    assert_eq!(
        float_clip.sample_format(),
        engine_audio::clip::SampleFormat::F32
    );
    assert_eq!(float_clip.f32_sample(0, 0), Some(0.25));

    for rate in [8_000, 192_000] {
        let bytes = wav(&[fmt_chunk(1, 1, rate, 16), pcm16(&[7])]);
        let clip = import_wav(&bytes).unwrap();
        assert_eq!(clip.sample_rate_hz(), rate);
        assert_eq!(clip.channel_count(), 1);
        assert_eq!(clip.i16_sample(0, 0), Some(7));
    }
}
