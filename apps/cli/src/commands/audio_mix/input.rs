use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use engine_audio::clip::{PcmClip, SampleFormat};
use engine_audio::wav::import_wav;

use super::error::AudioMixError;

pub(super) const MAX_INPUT_BYTES: usize = 16 * 1024 * 1024;
pub(super) const MAX_TOTAL_INPUT_BYTES: usize = 64 * 1024 * 1024;
pub(super) const MAX_OUTPUT_FRAMES: u64 = 1_048_576;

#[derive(Clone)]
pub(super) struct SourceSpec {
    pub(super) path: PathBuf,
    pub(super) start_frame: u64,
    pub(super) gain: f32,
}

pub(super) struct LoadedSource {
    pub(super) spec: SourceSpec,
    pub(super) clip: Arc<PcmClip>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) struct ClipFormat {
    pub(super) sample_rate_hz: u32,
    pub(super) channels: u16,
}

pub(super) fn load_sources(
    specs: &[SourceSpec],
) -> Result<(Vec<LoadedSource>, ClipFormat, u64), AudioMixError> {
    let mut total_bytes = 0usize;
    let mut sources = Vec::with_capacity(specs.len());
    let mut format: Option<ClipFormat> = None;
    let mut output_frames = 0u64;

    for spec in specs {
        let (source, source_format) = load_source(spec, &mut total_bytes)?;
        ensure_compatible_format(&mut format, &source.spec.path, source_format)?;
        output_frames = output_frames.max(source_end_frame(&source)?);
        sources.push(source);
    }

    Ok((
        sources,
        format.expect("at least one source was parsed"),
        output_frames,
    ))
}

fn load_source(
    spec: &SourceSpec,
    total_bytes: &mut usize,
) -> Result<(LoadedSource, ClipFormat), AudioMixError> {
    let bytes = read_input(&spec.path)?;
    add_input_bytes(total_bytes, bytes.len())?;

    let clip = import_wav(&bytes).map_err(|source| AudioMixError::Import {
        path: spec.path.clone(),
        source,
    })?;
    let format = validate_clip(&spec.path, &clip)?;
    let source = LoadedSource {
        spec: spec.clone(),
        clip: Arc::new(clip),
    };

    Ok((source, format))
}

fn add_input_bytes(total_bytes: &mut usize, input_bytes: usize) -> Result<(), AudioMixError> {
    *total_bytes = total_bytes
        .checked_add(input_bytes)
        .ok_or(AudioMixError::AggregateInputTooLarge)?;

    if *total_bytes > MAX_TOTAL_INPUT_BYTES {
        return Err(AudioMixError::AggregateInputTooLarge);
    }

    Ok(())
}

fn ensure_compatible_format(
    expected: &mut Option<ClipFormat>,
    path: &Path,
    actual: ClipFormat,
) -> Result<(), AudioMixError> {
    match expected {
        Some(expected) if *expected != actual => Err(AudioMixError::FormatMismatch {
            path: path.to_path_buf(),
        }),
        Some(_) => Ok(()),
        None => {
            *expected = Some(actual);
            Ok(())
        }
    }
}

fn source_end_frame(source: &LoadedSource) -> Result<u64, AudioMixError> {
    let frame_count = u64::try_from(source.clip.frame_count())
        .map_err(|_| AudioMixError::FrameCountOverflow(source.spec.path.clone()))?;
    let end_frame = source
        .spec
        .start_frame
        .checked_add(frame_count)
        .ok_or_else(|| AudioMixError::EndFrameOverflow(source.spec.path.clone()))?;

    if end_frame > MAX_OUTPUT_FRAMES {
        return Err(AudioMixError::OutputFrameLimit {
            path: source.spec.path.clone(),
            end_frame,
        });
    }

    Ok(end_frame)
}

fn read_input(path: &Path) -> Result<Vec<u8>, AudioMixError> {
    let file = File::open(path).map_err(|source| AudioMixError::InputOpen {
        path: path.to_path_buf(),
        source,
    })?;
    let mut reader = file.take((MAX_INPUT_BYTES + 1) as u64);
    let mut bytes = Vec::new();
    bytes
        .try_reserve_exact(MAX_INPUT_BYTES + 1)
        .map_err(|_| AudioMixError::InputAllocationFailed(path.to_path_buf()))?;
    reader
        .read_to_end(&mut bytes)
        .map_err(|source| AudioMixError::InputRead {
            path: path.to_path_buf(),
            source,
        })?;
    if bytes.len() > MAX_INPUT_BYTES {
        return Err(AudioMixError::InputTooLarge(path.to_path_buf()));
    }
    Ok(bytes)
}

fn validate_clip(path: &Path, clip: &PcmClip) -> Result<ClipFormat, AudioMixError> {
    if clip.sample_format() != SampleFormat::F32 {
        return Err(AudioMixError::UnsupportedSampleFormat {
            path: path.to_path_buf(),
            format: clip.sample_format(),
        });
    }
    Ok(ClipFormat {
        sample_rate_hz: clip.sample_rate_hz(),
        channels: clip.channel_count(),
    })
}
