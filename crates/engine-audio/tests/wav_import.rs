//! Public acceptance for bounded RIFF/WAVE parsing and exact PCM samples.

#[path = "wav_import/common.rs"]
mod common;
#[path = "wav_import/container.rs"]
mod container;
#[path = "wav_import/malformed_and_limits.rs"]
mod malformed_and_limits;
#[path = "wav_import/valid.rs"]
mod valid;
