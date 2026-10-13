use std::fmt;
use std::num::{NonZeroU16, NonZeroU32};

/// The sample representation retained by a validated clip.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SampleFormat {
    /// Normalized finite 32-bit floating-point samples.
    F32,
    /// Signed 16-bit integer samples retained without conversion.
    I16,
}

/// Owned interleaved storage for one PCM format.
#[derive(Clone, Debug, PartialEq)]
enum InterleavedPcm {
    F32(Box<[f32]>),
    I16(Box<[i16]>),
}

impl InterleavedPcm {
    fn format(&self) -> SampleFormat {
        match self {
            Self::F32(_) => SampleFormat::F32,
            Self::I16(_) => SampleFormat::I16,
        }
    }
}

/// A finite, immutable-after-validation interleaved PCM clip.
#[derive(Clone, Debug, PartialEq)]
pub struct PcmClip {
    sample_rate_hz: NonZeroU32,
    channel_count: NonZeroU16,
    frame_count: usize,
    samples: InterleavedPcm,
}

/// Failures produced while admitting PCM clip data.
#[derive(Clone, Debug, PartialEq)]
pub enum PcmClipError {
    /// The supplied sample rate was zero.
    ZeroSampleRate,
    /// The supplied channel count was zero.
    ZeroChannels,
    /// The sample buffer contained no samples.
    EmptyData,
    /// The sample count was not divisible by the channel count.
    MisalignedInterleave {
        /// Number of supplied interleaved samples.
        sample_count: usize,
        /// Number of channels used to form a frame.
        channel_count: u16,
    },
    /// Checked frame arithmetic could not represent the sample count.
    LengthOverflow,
    /// An F32 sample was NaN or infinite.
    NonFiniteSample {
        /// Zero-based position of the invalid sample.
        index: usize,
        /// Invalid non-finite value.
        value: f32,
    },
    /// An F32 sample was finite but outside the normalized range.
    SampleOutOfRange {
        /// Zero-based position of the out-of-range sample.
        index: usize,
        /// Finite value outside `[-1.0, 1.0]`.
        value: f32,
    },
}
impl PcmClip {
    /// Copies finite interleaved `f32` samples into a validated clip.
    pub fn from_f32(
        sample_rate_hz: u32,
        channel_count: u16,
        samples: &[f32],
    ) -> Result<Self, PcmClipError> {
        let (sample_rate_hz, channel_count, frame_count) =
            validate_metadata(sample_rate_hz, channel_count, samples.len())?;
        validate_f32_samples(samples)?;

        Ok(Self {
            sample_rate_hz,
            channel_count,
            frame_count,
            samples: InterleavedPcm::F32(samples.to_vec().into_boxed_slice()),
        })
    }

    /// Copies interleaved `i16` samples into a validated clip.
    pub fn from_i16(
        sample_rate_hz: u32,
        channel_count: u16,
        samples: &[i16],
    ) -> Result<Self, PcmClipError> {
        let (sample_rate_hz, channel_count, frame_count) =
            validate_metadata(sample_rate_hz, channel_count, samples.len())?;

        Ok(Self {
            sample_rate_hz,
            channel_count,
            frame_count,
            samples: InterleavedPcm::I16(samples.to_vec().into_boxed_slice()),
        })
    }

    /// Returns the positive source sample rate in hertz.
    pub fn sample_rate_hz(&self) -> u32 {
        self.sample_rate_hz.get()
    }

    /// Returns the positive number of samples in each frame.
    pub fn channel_count(&self) -> u16 {
        self.channel_count.get()
    }

    /// Returns the retained sample representation.
    pub fn sample_format(&self) -> SampleFormat {
        self.samples.format()
    }

    /// Returns the exact number of complete interleaved frames.
    pub fn frame_count(&self) -> usize {
        self.frame_count
    }

    /// Returns the finite duration derived from frame count and sample rate.
    pub fn duration_seconds(&self) -> f64 {
        self.frame_count as f64 / f64::from(self.sample_rate_hz())
    }

    /// Borrows the F32 storage when this clip has F32 format.
    pub fn f32_samples(&self) -> Option<&[f32]> {
        match &self.samples {
            InterleavedPcm::F32(samples) => Some(samples),
            InterleavedPcm::I16(_) => None,
        }
    }

    /// Borrows the I16 storage when this clip has I16 format.
    pub fn i16_samples(&self) -> Option<&[i16]> {
        match &self.samples {
            InterleavedPcm::F32(_) => None,
            InterleavedPcm::I16(samples) => Some(samples),
        }
    }

    /// Returns a sample by frame and channel without converting its format.
    pub fn sample_index(&self, frame: usize, channel: usize) -> Option<usize> {
        if frame >= self.frame_count || channel >= usize::from(self.channel_count()) {
            return None;
        }

        frame
            .checked_mul(usize::from(self.channel_count()))
            .and_then(|offset| offset.checked_add(channel))
    }

    /// Returns one F32 sample by frame and channel without conversion.
    pub fn f32_sample(&self, frame: usize, channel: usize) -> Option<f32> {
        let index = self.sample_index(frame, channel)?;
        self.f32_samples()?.get(index).copied()
    }

    /// Returns one I16 sample by frame and channel without conversion.
    pub fn i16_sample(&self, frame: usize, channel: usize) -> Option<i16> {
        let index = self.sample_index(frame, channel)?;
        self.i16_samples()?.get(index).copied()
    }
}

fn validate_metadata(
    sample_rate_hz: u32,
    channel_count: u16,
    sample_count: usize,
) -> Result<(NonZeroU32, NonZeroU16, usize), PcmClipError> {
    let sample_rate_hz = NonZeroU32::new(sample_rate_hz).ok_or(PcmClipError::ZeroSampleRate)?;
    let channel_count = NonZeroU16::new(channel_count).ok_or(PcmClipError::ZeroChannels)?;

    if sample_count == 0 {
        return Err(PcmClipError::EmptyData);
    }

    let channel_count_usize = usize::from(channel_count.get());
    if sample_count % channel_count_usize != 0 {
        return Err(PcmClipError::MisalignedInterleave {
            sample_count,
            channel_count: channel_count.get(),
        });
    }

    let frame_count = sample_count / channel_count_usize;
    let checked_sample_count = frame_count
        .checked_mul(channel_count_usize)
        .ok_or(PcmClipError::LengthOverflow)?;
    if checked_sample_count != sample_count {
        return Err(PcmClipError::LengthOverflow);
    }

    Ok((sample_rate_hz, channel_count, frame_count))
}

fn validate_f32_samples(samples: &[f32]) -> Result<(), PcmClipError> {
    for (index, &sample) in samples.iter().enumerate() {
        if !sample.is_finite() {
            return Err(PcmClipError::NonFiniteSample {
                index,
                value: sample,
            });
        }

        if !(-1.0..=1.0).contains(&sample) {
            return Err(PcmClipError::SampleOutOfRange {
                index,
                value: sample,
            });
        }
    }

    Ok(())
}

impl fmt::Display for PcmClipError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ZeroSampleRate => formatter.write_str("sample rate must be positive"),
            Self::ZeroChannels => formatter.write_str("channel count must be positive"),
            Self::EmptyData => formatter.write_str("PCM data must not be empty"),
            Self::MisalignedInterleave {
                sample_count,
                channel_count,
            } => write!(
                formatter,
                "{sample_count} samples cannot form complete frames with {channel_count} channels"
            ),
            Self::LengthOverflow => formatter.write_str("PCM frame arithmetic overflowed"),
            Self::NonFiniteSample { index, value } => {
                write!(formatter, "sample {index} is non-finite: {value:?}")
            }
            Self::SampleOutOfRange { index, value } => {
                write!(
                    formatter,
                    "sample {index} is outside normalized range: {value:?}"
                )
            }
        }
    }
}

impl std::error::Error for PcmClipError {}
