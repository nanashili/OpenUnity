//! Validated PCM clips, deterministic voice mixing, and offline DSP scheduling.

/// Validated interleaved PCM clip types and constructors.
pub mod clip;

/// Bounded deterministic voice admission and PCM mixing.
pub mod mixer;

/// Block-clock scheduling and bounded offline output publication.
pub mod offline;

/// Bounded RIFF/WAVE decoding into validated PCM clips.
pub mod wav;

pub use clip::{PcmClip, PcmClipError, SampleFormat};
