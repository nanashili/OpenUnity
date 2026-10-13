//! Bounded F32 WAVE rendering through the reviewed offline audio path.

use std::sync::Arc;

use crate::clip::{PcmClip, SampleFormat};
use crate::mixer::{MixFormat, VoiceRequest};
use crate::wav::{WavError, import_wav};

use super::{DspCommand, OfflineError, OfflineOutput, VoiceAdapterError, VoiceMixerAdapter};

/// Failure while importing and rendering an in-memory F32 WAVE preview.
#[derive(Clone, Debug, PartialEq)]
pub enum WavOfflineError {
    /// The WAVE container or samples were rejected by the importer.
    Import(WavError),
    /// The importer produced a PCM16 clip, which this path does not convert.
    UnsupportedSampleFormat(SampleFormat),
    /// The clip frame count cannot be represented by the offline clock.
    FrameCountOverflow,
    /// The bounded voice adapter rejected setup or preparation.
    Adapter(VoiceAdapterError),
    /// Scheduling, rendering, or publication failed.
    Renderer(OfflineError<VoiceAdapterError>),
}

/// Imports and renders a complete supported F32 WAVE as immutable output.
pub fn render_f32_wav_offline(
    bytes: &[u8],
    block_size: u32,
) -> Result<Arc<OfflineOutput>, WavOfflineError> {
    let clip = import_wav(bytes).map_err(WavOfflineError::Import)?;
    ensure_f32(&clip)?;
    let frame_count =
        u64::try_from(clip.frame_count()).map_err(|_| WavOfflineError::FrameCountOverflow)?;
    let clip = Arc::new(clip);
    let mut renderer = prepare_renderer(clip, block_size, frame_count)?;

    renderer
        .schedule(0, DspCommand::StartVoice { voice_id: 1 })
        .map_err(WavOfflineError::Renderer)?;
    renderer
        .render(frame_count)
        .map_err(WavOfflineError::Renderer)?;
    renderer.finish().map_err(WavOfflineError::Renderer)
}

fn ensure_f32(clip: &PcmClip) -> Result<(), WavOfflineError> {
    if clip.sample_format() == SampleFormat::F32 {
        Ok(())
    } else {
        Err(WavOfflineError::UnsupportedSampleFormat(
            clip.sample_format(),
        ))
    }
}

fn prepare_renderer(
    clip: Arc<PcmClip>,
    block_size: u32,
    frame_count: u64,
) -> Result<super::OfflineRenderer<VoiceMixerAdapter>, WavOfflineError> {
    let format = MixFormat::new(
        clip.sample_rate_hz(),
        clip.channel_count(),
        SampleFormat::F32,
    )
    .map_err(|error| WavOfflineError::Adapter(VoiceAdapterError::Mixer(error)))?;
    let mut adapter = VoiceMixerAdapter::new(1, 1, format).map_err(WavOfflineError::Adapter)?;
    adapter
        .prepare_voice(
            1,
            VoiceRequest {
                clip: Arc::clone(&clip),
                gain: 1.0,
                start_frame: 0,
                priority: 0.0,
            },
        )
        .map_err(WavOfflineError::Adapter)?;
    adapter
        .into_offline_renderer(block_size, 1, frame_count)
        .map_err(WavOfflineError::Renderer)
}
