use super::common::{base, expect_error, fmt_chunk, pcm16, truncate_declared_tail, unknown, wav};
use engine_audio::wav::WavError;

#[test]
fn rejects_container_signatures_and_required_chunk_order() {
    for signature in [*b"RIFX", *b"RF64"] {
        let mut invalid = base();
        invalid[0..4].copy_from_slice(&signature);
        expect_error(&invalid, WavError::InvalidSignature);
    }

    let mut wrong_form = base();
    wrong_form[8..12].copy_from_slice(b"AVI ");
    expect_error(&wrong_form, WavError::InvalidSignature);

    let data_first = wav(&[pcm16(&[1]), fmt_chunk(1, 1, 48_000, 16)]);
    expect_error(&data_first, WavError::DataBeforeFormat);

    let missing_format = wav(&[pcm16(&[1])]);
    expect_error(&missing_format, WavError::DataBeforeFormat);
    let missing_data = wav(&[fmt_chunk(1, 1, 48_000, 16)]);
    expect_error(&missing_data, WavError::MissingData);
}

#[test]
fn rejects_duplicates_and_malformed_chunks_after_data() {
    let duplicate_format = wav(&[
        fmt_chunk(1, 1, 48_000, 16),
        fmt_chunk(1, 1, 48_000, 16),
        pcm16(&[1]),
    ]);
    expect_error(&duplicate_format, WavError::DuplicateFormat);

    let duplicate_data = wav(&[fmt_chunk(1, 1, 48_000, 16), pcm16(&[1]), pcm16(&[2])]);
    expect_error(&duplicate_data, WavError::DuplicateData);

    let mut after_data = base();
    let riff_size = u32::from_le_bytes(after_data[4..8].try_into().unwrap());
    after_data[4..8].copy_from_slice(&(riff_size + 8).to_le_bytes());
    after_data.extend_from_slice(b"JUNK\xFF\xFF\xFF\x7F");
    expect_error(&after_data, WavError::TruncatedChunkPayload);
}

#[test]
fn rejects_truncation_and_alignment_errors() {
    for length in 0..12 {
        expect_error(&base()[..length], WavError::TruncatedRiffHeader);
    }

    let mut too_small = base();
    too_small[4..8].copy_from_slice(&3u32.to_le_bytes());
    expect_error(&too_small, WavError::RiffExtentTooSmall);

    let mut trailing = base();
    trailing.push(0);
    expect_error(&trailing, WavError::TrailingInput);

    let mut short_riff = base();
    short_riff[4..8].copy_from_slice(&1000u32.to_le_bytes());
    expect_error(&short_riff, WavError::TruncatedRiff);

    let mut short_header = base();
    short_header[4..8].copy_from_slice(&11u32.to_le_bytes());
    short_header.truncate(19);
    expect_error(&short_header, WavError::TruncatedChunkHeader);

    for remaining in 1..=7 {
        let mut bytes = wav(&[fmt_chunk(1, 1, 48_000, 16)]);
        let riff_size = u32::from_le_bytes(bytes[4..8].try_into().unwrap());
        bytes.extend(std::iter::repeat_n(0x4A, remaining));
        bytes[4..8].copy_from_slice(&(riff_size + remaining as u32).to_le_bytes());
        expect_error(&bytes, WavError::TruncatedChunkHeader);
    }

    let mut short_pad = wav(&[
        fmt_chunk(1, 1, 48_000, 16),
        pcm16(&[1]),
        unknown(b"JUNK", vec![1]),
    ]);
    let declared_size = u32::from_le_bytes(short_pad[4..8].try_into().unwrap());
    short_pad.truncate(short_pad.len() - 1);
    short_pad[4..8].copy_from_slice(&(declared_size - 1).to_le_bytes());
    expect_error(&short_pad, WavError::TruncatedChunkPad);

    for removed in 1..=4 {
        let truncated = truncate_declared_tail(
            wav(&[
                fmt_chunk(1, 1, 48_000, 16),
                pcm16(&[1]),
                unknown(b"JUNK", vec![1, 2, 3, 4]),
            ]),
            removed,
        );
        expect_error(&truncated, WavError::TruncatedChunkPayload);
    }

    for removed in 2..=3 {
        let truncated = truncate_declared_tail(
            wav(&[
                fmt_chunk(1, 1, 48_000, 16),
                pcm16(&[1]),
                unknown(b"JUNK", vec![1, 2, 3]),
            ]),
            removed,
        );
        expect_error(&truncated, WavError::TruncatedChunkPayload);
    }

    let truncated_pad = truncate_declared_tail(
        wav(&[
            fmt_chunk(1, 1, 48_000, 16),
            pcm16(&[1]),
            unknown(b"JUNK", vec![1, 2, 3]),
        ]),
        1,
    );
    expect_error(&truncated_pad, WavError::TruncatedChunkPad);

    let misaligned = wav(&[fmt_chunk(1, 2, 48_000, 16), unknown(b"data", vec![1, 2, 3])]);
    expect_error(&misaligned, WavError::DataNotFrameAligned);

    let pcm16_partial = wav(&[fmt_chunk(1, 1, 48_000, 16), unknown(b"data", vec![1])]);
    expect_error(&pcm16_partial, WavError::DataNotFrameAligned);

    let empty = wav(&[fmt_chunk(1, 1, 48_000, 16), unknown(b"data", vec![])]);
    expect_error(&empty, WavError::EmptyData);
}
