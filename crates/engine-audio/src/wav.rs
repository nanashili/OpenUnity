//! Bounded little-endian RIFF/WAVE import into validated PCM clips.

mod decode;
mod error;
mod structure;

use crate::clip::PcmClip;

pub use error::WavError;
use structure::scan;

/// Parses one bounded RIFF/WAVE file into an owned, validated PCM clip.
pub fn import_wav(bytes: &[u8]) -> Result<PcmClip, WavError> {
    let layout = scan(bytes)?;
    decode::decode_clip(bytes, layout)
}
