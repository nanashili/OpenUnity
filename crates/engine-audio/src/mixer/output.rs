use crate::clip::{PcmClip, SampleFormat};

use super::MixError;

#[derive(Clone, Debug, PartialEq)]
/// Interleaved samples produced by [`super::VoiceMixer::render`].
pub enum MixedOutput {
    /// Normalized floating-point samples in the range `[-1.0, 1.0]`.
    F32(Box<[f32]>),
    /// Signed 16-bit PCM samples using the configured output format.
    I16(Box<[i16]>),
}

pub(super) fn source_sample(clip: &PcmClip, frame: usize, channel: usize) -> Result<f64, MixError> {
    let value = match clip.sample_format() {
        SampleFormat::F32 => f64::from(
            clip.f32_sample(frame, channel)
                .ok_or(MixError::NonFiniteOutput)?,
        ),
        SampleFormat::I16 => {
            let sample = clip
                .i16_sample(frame, channel)
                .ok_or(MixError::NonFiniteOutput)?;
            if sample < 0 {
                f64::from(sample) / 32768.0
            } else {
                f64::from(sample) / 32767.0
            }
        }
    };
    value
        .is_finite()
        .then_some(value)
        .ok_or(MixError::NonFiniteOutput)
}

pub(super) fn finalize(sums: Vec<f64>, format: SampleFormat) -> Result<MixedOutput, MixError> {
    if sums.iter().any(|sum| !sum.is_finite()) {
        return Err(MixError::NonFiniteOutput);
    }
    match format {
        SampleFormat::F32 => Ok(MixedOutput::F32(
            sums.into_iter()
                .map(|sum| sum.clamp(-1.0, 1.0) as f32)
                .collect(),
        )),
        SampleFormat::I16 => Ok(MixedOutput::I16(
            sums.into_iter().map(to_i16).collect::<Result<_, _>>()?,
        )),
    }
}

fn to_i16(sum: f64) -> Result<i16, MixError> {
    let clipped = sum.clamp(-1.0, 1.0);
    let scaled = if clipped < 0.0 {
        clipped * 32768.0
    } else {
        clipped * 32767.0
    };
    i16::try_from(round_ties_even(scaled) as i32).map_err(|_| MixError::NonFiniteOutput)
}

fn round_ties_even(value: f64) -> f64 {
    let lower = value.floor();
    let fraction = value - lower;
    if fraction < 0.5 || (fraction == 0.5 && (lower as i64) % 2 == 0) {
        lower
    } else {
        lower + 1.0
    }
}
