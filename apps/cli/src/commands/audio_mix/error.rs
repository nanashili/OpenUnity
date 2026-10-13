use std::fmt;
use std::io;
use std::path::PathBuf;

use engine_audio::SampleFormat;

use crate::commands::audio_output;

#[derive(Debug)]
pub(in crate::commands) enum AudioMixError {
    Usage,
    TooManyInputs(usize),
    InvalidBlockFrames(String),
    InvalidStartFrame {
        path: PathBuf,
        value: String,
    },
    InvalidGain {
        path: PathBuf,
        value: String,
    },
    InputOpen {
        path: PathBuf,
        source: io::Error,
    },
    InputRead {
        path: PathBuf,
        source: io::Error,
    },
    InputAllocationFailed(PathBuf),
    InputTooLarge(PathBuf),
    AggregateInputTooLarge,
    Import {
        path: PathBuf,
        source: engine_audio::wav::WavError,
    },
    UnsupportedSampleFormat {
        path: PathBuf,
        format: SampleFormat,
    },
    FormatMismatch {
        path: PathBuf,
    },
    FrameCountOverflow(PathBuf),
    EndFrameOverflow(PathBuf),
    OutputFrameLimit {
        path: PathBuf,
        end_frame: u64,
    },
    Adapter(engine_audio::offline::VoiceAdapterError),
    Renderer(engine_audio::offline::OfflineError<engine_audio::offline::VoiceAdapterError>),
    Output(audio_output::WaveFileError),
}

impl fmt::Display for AudioMixError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Usage => formatter.write_str(
                "usage: openunity-cli --mix-wav OUTPUT.wav BLOCK_FRAMES INPUT.wav START_FRAME GAIN [INPUT.wav START_FRAME GAIN ...]",
            ),
            Self::TooManyInputs(count) => {
                write!(formatter, "received {count} inputs; the maximum is 8")
            }
            Self::InvalidBlockFrames(value) => {
                write!(formatter, "invalid BLOCK_FRAMES value {value:?}; expected positive u32")
            }
            Self::InvalidStartFrame { path, value } => {
                write!(formatter, "invalid start frame {value:?} for {}", path.display())
            }
            Self::InvalidGain { path, value } => {
                write!(formatter, "invalid gain {value:?} for {}", path.display())
            }
            Self::InputOpen { path, source } => {
                write!(formatter, "cannot open input {}: {source}", path.display())
            }
            Self::InputRead { path, source } => {
                write!(formatter, "cannot read input {}: {source}", path.display())
            }
            Self::InputAllocationFailed(path) => write!(
                formatter,
                "cannot reserve the bounded input buffer for {}",
                path.display()
            ),
            Self::InputTooLarge(path) => write!(
                formatter,
                "input {} exceeds the 16 MiB WAVE import limit",
                path.display()
            ),
            Self::AggregateInputTooLarge => {
                formatter.write_str("aggregate WAVE input exceeds the 64 MiB limit")
            }
            Self::Import { path, source } => {
                write!(formatter, "cannot import {}: {source:?}", path.display())
            }
            Self::UnsupportedSampleFormat { path, format } => write!(
                formatter,
                "input {} uses unsupported sample format {format:?}; only F32 is accepted",
                path.display()
            ),
            Self::FormatMismatch { path } => write!(
                formatter,
                "input {} does not match the first input sample rate and channel format",
                path.display()
            ),
            Self::FrameCountOverflow(path) => {
                write!(formatter, "frame count for {} does not fit u64", path.display())
            }
            Self::EndFrameOverflow(path) => write!(
                formatter,
                "start frame plus clip length overflowed for {}",
                path.display()
            ),
            Self::OutputFrameLimit { path, end_frame } => write!(
                formatter,
                "source {} ends at frame {end_frame}, beyond the 1,048,576-frame output limit",
                path.display()
            ),
            Self::Adapter(error) => write!(formatter, "audio mix setup failed: {error:?}"),
            Self::Renderer(error) => write!(formatter, "audio mix render failed: {error:?}"),
            Self::Output(error) => write!(formatter, "WAVE mix export failed: {error}"),
        }
    }
}

impl std::error::Error for AudioMixError {}
