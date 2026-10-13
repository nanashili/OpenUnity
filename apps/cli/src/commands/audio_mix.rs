//! Headless bounded multi-input offline F32 WAVE mixing.

use std::error::Error;
use std::path::PathBuf;

use engine_audio::SampleFormat;
use engine_audio::mixer::{MixFormat, VoiceRequest};
use engine_audio::offline::{DspCommand, VoiceMixerAdapter};

use super::audio_output;

mod error;
mod input;

use error::AudioMixError;
use input::{LoadedSource, MAX_OUTPUT_FRAMES, SourceSpec};

const MAX_INPUTS: usize = 8;

pub(super) fn run(args: &[String]) -> Result<(), Box<dyn Error>> {
    let request = MixRequest::parse(args)?;
    let (sources, format, output_frames) = input::load_sources(&request.sources)?;
    let output = render_sources(&sources, format, request.block_frames, output_frames)?;
    audio_output::publish(&request.output_path, &output).map_err(AudioMixError::Output)?;
    println!(
        "mixed {} inputs to {}",
        sources.len(),
        request.output_path.display()
    );
    Ok(())
}

struct MixRequest {
    output_path: PathBuf,
    block_frames: u32,
    sources: Vec<SourceSpec>,
}

impl MixRequest {
    fn parse(args: &[String]) -> Result<Self, AudioMixError> {
        if args.len() < 6 || args[0] != "--mix-wav" || (args.len() - 3) % 3 != 0 {
            return Err(AudioMixError::Usage);
        }
        let block_frames = args[2]
            .parse::<u32>()
            .ok()
            .filter(|frames| *frames > 0)
            .ok_or_else(|| AudioMixError::InvalidBlockFrames(args[2].clone()))?;
        let input_count = (args.len() - 3) / 3;
        if input_count > MAX_INPUTS {
            return Err(AudioMixError::TooManyInputs(input_count));
        }

        let mut sources = Vec::with_capacity(input_count);
        for values in args[3..].chunks_exact(3) {
            let path = PathBuf::from(&values[0]);
            let start_frame =
                values[1]
                    .parse::<u64>()
                    .map_err(|_| AudioMixError::InvalidStartFrame {
                        path: path.clone(),
                        value: values[1].clone(),
                    })?;
            let gain = values[2]
                .parse::<f32>()
                .ok()
                .filter(|gain| gain.is_finite() && *gain >= 0.0)
                .ok_or_else(|| AudioMixError::InvalidGain {
                    path: path.clone(),
                    value: values[2].clone(),
                })?;
            sources.push(SourceSpec {
                path,
                start_frame,
                gain,
            });
        }
        Ok(Self {
            output_path: PathBuf::from(&args[1]),
            block_frames,
            sources,
        })
    }
}

fn render_sources(
    sources: &[LoadedSource],
    format: input::ClipFormat,
    block_frames: u32,
    output_frames: u64,
) -> Result<std::sync::Arc<engine_audio::offline::OfflineOutput>, AudioMixError> {
    debug_assert!(output_frames > 0 && output_frames <= MAX_OUTPUT_FRAMES);
    let mix_format = MixFormat::new(format.sample_rate_hz, format.channels, SampleFormat::F32)
        .map_err(|error| {
            AudioMixError::Adapter(engine_audio::offline::VoiceAdapterError::Mixer(error))
        })?;
    let capacity = sources.len();
    let mut adapter =
        VoiceMixerAdapter::new(capacity, capacity, mix_format).map_err(AudioMixError::Adapter)?;

    for (index, source) in sources.iter().enumerate() {
        let schedule_id = u64::try_from(index + 1).expect("bounded source count fits u64");
        adapter
            .prepare_voice(
                schedule_id,
                VoiceRequest {
                    clip: std::sync::Arc::clone(&source.clip),
                    gain: source.spec.gain,
                    start_frame: source.spec.start_frame,
                    priority: 0.0,
                },
            )
            .map_err(AudioMixError::Adapter)?;
    }

    let mut renderer = adapter
        .into_offline_renderer(block_frames, capacity, output_frames)
        .map_err(AudioMixError::Renderer)?;
    for (index, source) in sources.iter().enumerate() {
        let schedule_id = u64::try_from(index + 1).expect("bounded source count fits u64");
        renderer
            .schedule(
                source.spec.start_frame,
                DspCommand::StartVoice {
                    voice_id: schedule_id,
                },
            )
            .map_err(AudioMixError::Renderer)?;
    }
    renderer
        .render(output_frames)
        .map_err(AudioMixError::Renderer)?;
    renderer.finish().map_err(AudioMixError::Renderer)
}
