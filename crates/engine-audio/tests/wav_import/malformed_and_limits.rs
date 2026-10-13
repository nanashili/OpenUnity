use super::common::{
    expect_error, float32, fmt_chunk, fmt_extension, pcm16, truncate_declared_tail, unknown, wav,
};
use engine_audio::clip::PcmClipError;
use engine_audio::wav::{WavError, import_wav};

#[test]
fn rejects_format_rules_and_supported_profile_boundaries() {
    for length in 0usize..=15 {
        let mut chunk = fmt_chunk(1, 1, 48_000, 16);
        chunk.payload.truncate(length);
        expect_error(&wav(&[chunk, pcm16(&[1])]), WavError::InvalidFormatLength);
    }

    let mut partial_extension = fmt_extension(0);
    partial_extension.payload.truncate(17);
    expect_error(
        &wav(&[partial_extension, pcm16(&[1])]),
        WavError::InvalidFormatLength,
    );

    let mut oversized = fmt_chunk(1, 1, 48_000, 16);
    oversized.payload.extend_from_slice(&[0, 0, 0]);
    expect_error(
        &wav(&[oversized, pcm16(&[1])]),
        WavError::UnsupportedFormatExtension,
    );

    expect_error(
        &wav(&[fmt_chunk(1, 1, 48_000, 8), pcm16(&[1])]),
        WavError::UnsupportedBitDepth(8),
    );
    expect_error(
        &wav(&[fmt_chunk(2, 1, 48_000, 16), pcm16(&[1])]),
        WavError::UnsupportedFormatTag(2),
    );
    expect_error(
        &wav(&[fmt_chunk(0xFFFE, 1, 48_000, 16), pcm16(&[1])]),
        WavError::UnsupportedFormatTag(0xFFFE),
    );
    expect_error(
        &wav(&[fmt_chunk(1, 3, 48_000, 16), pcm16(&[1, 2, 3])]),
        WavError::UnsupportedChannelCount(3),
    );
    expect_error(
        &wav(&[fmt_chunk(1, 1, 7_999, 16), pcm16(&[1])]),
        WavError::UnsupportedSampleRate(7_999),
    );
    expect_error(
        &wav(&[fmt_chunk(1, 1, 192_001, 16), pcm16(&[1])]),
        WavError::UnsupportedSampleRate(192_001),
    );

    let mut bad_align = fmt_chunk(1, 1, 48_000, 16);
    bad_align.payload[12..14].copy_from_slice(&4u16.to_le_bytes());
    expect_error(
        &wav(&[bad_align, pcm16(&[1])]),
        WavError::InconsistentBlockAlignment,
    );

    let mut bad_rate = fmt_chunk(1, 1, 48_000, 16);
    bad_rate.payload[8..12].copy_from_slice(&1u32.to_le_bytes());
    expect_error(
        &wav(&[bad_rate, pcm16(&[1])]),
        WavError::InconsistentByteRate,
    );

    expect_error(
        &wav(&[fmt_extension(1), pcm16(&[1])]),
        WavError::UnsupportedFormatExtension,
    );

    expect_error(
        &wav(&[fmt_chunk(1, 0, 48_000, 16), pcm16(&[1])]),
        WavError::UnsupportedChannelCount(0),
    );
    expect_error(
        &wav(&[fmt_chunk(1, 1, 0, 16), pcm16(&[1])]),
        WavError::UnsupportedSampleRate(0),
    );
    expect_error(
        &wav(&[fmt_chunk(3, 1, 48_000, 16), pcm16(&[1])]),
        WavError::UnsupportedBitDepth(16),
    );
}

#[test]
fn wraps_nonfinite_and_out_of_range_float_samples() {
    assert!(matches!(
        import_wav(&wav(&[fmt_chunk(3, 1, 48_000, 32), float32(&[f32::NAN])])),
        Err(WavError::PcmClip(PcmClipError::NonFiniteSample {
            index: 0,
            ..
        }))
    ));
    assert!(matches!(
        import_wav(&wav(&[
            fmt_chunk(3, 1, 48_000, 32),
            float32(&[f32::INFINITY])
        ])),
        Err(WavError::PcmClip(PcmClipError::NonFiniteSample {
            index: 0,
            ..
        }))
    ));
    assert!(matches!(
        import_wav(&wav(&[
            fmt_chunk(3, 1, 48_000, 32),
            float32(&[f32::from_bits(0x7F80_0001)])
        ])),
        Err(WavError::PcmClip(PcmClipError::NonFiniteSample {
            index: 0,
            ..
        }))
    ));
    assert!(matches!(
        import_wav(&wav(&[
            fmt_chunk(3, 1, 48_000, 32),
            float32(&[f32::NEG_INFINITY])
        ])),
        Err(WavError::PcmClip(PcmClipError::NonFiniteSample {
            index: 0,
            ..
        }))
    ));
    assert!(matches!(
        import_wav(&wav(&[fmt_chunk(3, 1, 48_000, 32), float32(&[-1.0000001])])),
        Err(WavError::PcmClip(PcmClipError::SampleOutOfRange {
            index: 0,
            ..
        }))
    ));
    expect_error(
        &wav(&[fmt_chunk(3, 1, 48_000, 32), float32(&[1.0000001])]),
        WavError::PcmClip(PcmClipError::SampleOutOfRange {
            index: 0,
            value: 1.0000001,
        }),
    );

    assert!(matches!(
        import_wav(&wav(&[
            fmt_chunk(3, 1, 48_000, 32),
            float32(&[0.0, 1.0000001])
        ])),
        Err(WavError::PcmClip(PcmClipError::SampleOutOfRange {
            index: 1,
            value: 1.0000001,
        }))
    ));
}

#[test]
fn preserves_negative_zero_float_bits() {
    let clip = import_wav(&wav(&[fmt_chunk(3, 1, 48_000, 32), float32(&[-0.0])])).unwrap();

    assert_eq!(
        clip.f32_sample(0, 0).unwrap().to_bits(),
        (-0.0f32).to_bits()
    );
}

#[test]
fn accepts_exact_frame_ceiling_and_rejects_one_more() {
    let exact = vec![0i16; 1_048_576];
    let exact_file = wav(&[fmt_chunk(1, 1, 48_000, 16), pcm16(&exact)]);
    let clip = import_wav(&exact_file).unwrap();
    assert_eq!(clip.sample_format(), engine_audio::clip::SampleFormat::I16);
    assert_eq!(clip.channel_count(), 1);
    assert_eq!(clip.frame_count(), 1_048_576);
    assert_eq!(clip.i16_sample(1_048_575, 0), Some(0));

    let over = vec![0i16; 1_048_577];
    expect_error(
        &wav(&[fmt_chunk(1, 1, 48_000, 16), pcm16(&over)]),
        WavError::FrameLimitExceeded,
    );
}

#[test]
fn rejects_data_truncation_before_sample_and_frame_boundaries() {
    let complete = wav(&[fmt_chunk(1, 1, 48_000, 16), pcm16(&[1, 2])]);
    for removed in 1..=4 {
        let truncated = truncate_declared_tail(complete.clone(), removed);
        expect_error(&truncated, WavError::TruncatedChunkPayload);
    }

    let float_complete = wav(&[fmt_chunk(3, 1, 48_000, 32), float32(&[0.0, 0.5])]);
    for removed in 1..=4 {
        let truncated = truncate_declared_tail(float_complete.clone(), removed);
        expect_error(&truncated, WavError::TruncatedChunkPayload);
    }

    let float_misaligned = wav(&[fmt_chunk(3, 1, 48_000, 32), unknown(b"data", vec![0, 0, 0])]);
    expect_error(&float_misaligned, WavError::DataNotFrameAligned);
}

#[test]
fn accepts_exact_data_ceiling_and_rejects_one_more() {
    let exact = vec![0.0; 2 * 1_048_576];
    let exact_file = wav(&[fmt_chunk(3, 2, 48_000, 32), float32(&exact)]);
    assert_eq!(exact_file.len(), 8 * 1024 * 1024 + 44);
    let clip = import_wav(&exact_file).unwrap();
    assert_eq!(clip.sample_format(), engine_audio::clip::SampleFormat::F32);
    assert_eq!(clip.channel_count(), 2);
    assert_eq!(clip.frame_count(), 1_048_576);
    assert_eq!(clip.f32_sample(1_048_575, 1), Some(0.0));

    let over = vec![0.0; 2 * 1_048_577];
    expect_error(
        &wav(&[fmt_chunk(3, 2, 48_000, 32), float32(&over)]),
        WavError::DataTooLarge,
    );
}

#[test]
fn accepts_exact_chunk_ceiling_and_rejects_one_more() {
    let mut chunks = vec![fmt_chunk(1, 1, 48_000, 16), pcm16(&[1])];
    chunks.extend((0..4_094).map(|_| unknown(b"JUNK", vec![])));
    let clip = import_wav(&wav(&chunks)).unwrap();
    assert_eq!(clip.sample_rate_hz(), 48_000);
    assert_eq!(clip.channel_count(), 1);
    assert_eq!(clip.i16_sample(0, 0), Some(1));
    chunks.push(unknown(b"JUNK", vec![]));
    expect_error(&wav(&chunks), WavError::ChunkCountExceeded);
}

#[test]
fn enforces_exact_input_ceiling_before_parsing() {
    let payload_len = 16 * 1024 * 1024 - 54;
    let exact = wav(&[
        fmt_chunk(1, 1, 48_000, 16),
        pcm16(&[1234]),
        unknown(b"JUNK", vec![0; payload_len]),
    ]);
    assert_eq!(exact.len(), 16 * 1024 * 1024);

    let clip = import_wav(&exact).unwrap();
    assert_eq!(clip.i16_samples(), Some(&[1234][..]));

    let mut over = exact.clone();
    let riff_size = u32::from_le_bytes(over[4..8].try_into().unwrap());
    let junk_size_offset = 50;
    let junk_size = u32::from_le_bytes(
        over[junk_size_offset..junk_size_offset + 4]
            .try_into()
            .unwrap(),
    );
    over[junk_size_offset..junk_size_offset + 4].copy_from_slice(&(junk_size + 2).to_le_bytes());
    over.extend_from_slice(&[0, 0]);
    over[4..8].copy_from_slice(&(riff_size + 2).to_le_bytes());
    assert_eq!(over.len(), 16 * 1024 * 1024 + 2);
    expect_error(&over, WavError::InputTooLarge);
}
