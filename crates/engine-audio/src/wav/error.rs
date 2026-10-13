use crate::clip::PcmClipError;

/// Failure while validating or decoding a bounded RIFF/WAVE file.
#[derive(Clone, Debug, PartialEq)]
pub enum WavError {
    /// The source exceeds the importer input limit.
    InputTooLarge,
    /// The RIFF header is incomplete.
    TruncatedRiffHeader,
    /// The RIFF or WAVE identifiers are invalid.
    InvalidSignature,
    /// The declared RIFF form is too small to contain `WAVE`.
    RiffExtentTooSmall,
    /// The declared RIFF extent exceeds the physical input.
    TruncatedRiff,
    /// Bytes remain after the declared RIFF extent.
    TrailingInput,
    /// A chunk header is incomplete.
    TruncatedChunkHeader,
    /// A chunk payload exceeds the declared RIFF extent.
    TruncatedChunkPayload,
    /// An odd-sized chunk has no in-range alignment byte.
    TruncatedChunkPad,
    /// The top-level chunk count exceeds the importer limit.
    ChunkCountExceeded,
    /// A checked offset or size calculation overflowed.
    SizeOverflow,
    /// The required `fmt ` chunk is absent.
    MissingFormat,
    /// More than one `fmt ` chunk was present.
    DuplicateFormat,
    /// The required `data` chunk is absent.
    MissingData,
    /// More than one `data` chunk was present.
    DuplicateData,
    /// A `data` chunk occurred before an accepted `fmt ` chunk.
    DataBeforeFormat,
    /// The `fmt ` payload is not one of the supported lengths.
    InvalidFormatLength,
    /// The WAVE format tag is unsupported.
    UnsupportedFormatTag(u16),
    /// The sample width is unsupported for the format tag.
    UnsupportedBitDepth(u16),
    /// The WAVE format extension is unsupported or non-empty.
    UnsupportedFormatExtension,
    /// The channel count is outside the supported mono/stereo profile.
    UnsupportedChannelCount(u16),
    /// The sample rate is outside the supported engine range.
    UnsupportedSampleRate(u32),
    /// `nBlockAlign` does not match the format metadata.
    InconsistentBlockAlignment,
    /// `nAvgBytesPerSec` does not match the format metadata.
    InconsistentByteRate,
    /// The data chunk contains no bytes.
    EmptyData,
    /// The data byte count is not a whole number of frames.
    DataNotFrameAligned,
    /// The data payload exceeds the importer byte limit.
    DataTooLarge,
    /// The frame count exceeds the importer frame limit.
    FrameLimitExceeded,
    /// The decoded sample buffer could not be reserved.
    SampleAllocationFailed,
    /// The constructed clip rejected metadata or a decoded sample.
    PcmClip(PcmClipError),
}
