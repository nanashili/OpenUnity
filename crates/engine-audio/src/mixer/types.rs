use std::sync::Arc;

use crate::clip::{PcmClip, SampleFormat};

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
/// Stable identity assigned to an admitted voice.
pub struct VoiceId(pub(crate) u64);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
/// Output format required by a [`super::VoiceMixer`].
pub struct MixFormat {
    pub(crate) sample_rate_hz: u32,
    pub(crate) channel_count: u16,
    pub(crate) sample_format: SampleFormat,
}

impl MixFormat {
    /// Creates a format with a non-zero sample rate and channel count.
    pub fn new(
        sample_rate_hz: u32,
        channel_count: u16,
        sample_format: SampleFormat,
    ) -> Result<Self, super::MixError> {
        if sample_rate_hz == 0 || channel_count == 0 {
            return Err(super::MixError::UnsupportedOutputFormat);
        }
        Ok(Self {
            sample_rate_hz,
            channel_count,
            sample_format,
        })
    }
}

#[derive(Clone, Debug)]
/// A clip and playback parameters submitted for admission.
pub struct VoiceRequest {
    /// The validated PCM clip to mix.
    pub clip: Arc<PcmClip>,
    /// Linear amplitude multiplier applied to the clip.
    pub gain: f32,
    /// Mixer frame at which playback begins.
    pub start_frame: u64,
    /// Relative priority used for bounded-capacity admission.
    pub priority: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
/// Public state for an admitted voice.
pub struct VoiceSnapshot {
    /// Stable identity of the voice.
    pub id: VoiceId,
    /// Mixer frame at which playback begins.
    pub start_frame: u64,
    /// Relative priority used for eviction decisions.
    pub priority: f32,
}

#[derive(Clone, Debug, PartialEq)]
/// Failure returned when a mixer operation cannot be completed.
pub enum MixError {
    /// The mixer was created with no admission capacity.
    ZeroCapacity,
    /// The new voice does not outrank any voice currently admitted.
    CapacityExceeded,
    /// The request gain is not finite.
    NonFiniteGain,
    /// The request gain is negative.
    NegativeGain,
    /// The request priority is not finite.
    NonFinitePriority,
    /// The request start frame is invalid.
    InvalidStartFrame,
    /// Rendering was requested for no frames.
    ZeroRenderLength,
    /// A frame or sample count exceeded the supported range.
    FrameIndexOverflow,
    /// The request starts before the mixer’s current frame.
    PastFrame,
    /// A source sample or accumulated output was not finite.
    NonFiniteOutput,
    /// The requested output format has zero dimensions.
    UnsupportedOutputFormat,
    /// The clip format differs from the mixer format.
    SourceFormatMismatch,
    /// The requested voice identity is not currently active.
    UnknownVoice(VoiceId),
}
