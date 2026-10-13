//! Headless export of an in-memory offline F32 WAVE render.

use std::error::Error;
use std::fmt;
use std::fs::File;
use std::io::{self, Read};
use std::path::{Path, PathBuf};

use engine_audio::offline::render_f32_wav_offline;

use super::audio_output;

const MAX_INPUT_BYTES: usize = 16 * 1024 * 1024;

/// Runs `--export-wav INPUT.wav OUTPUT.wav BLOCK_FRAMES`.
pub(super) fn run(args: &[String]) -> Result<(), Box<dyn Error>> {
    let request = ExportRequest::parse(args)?;
    let bytes = read_input(&request.input_path)?;
    let output =
        render_f32_wav_offline(&bytes, request.block_frames).map_err(AudioExportError::Render)?;
    audio_output::publish(&request.output_path, &output).map_err(AudioExportError::Output)?;
    println!(
        "exported {} to {}",
        request.input_path.display(),
        request.output_path.display()
    );
    Ok(())
}

struct ExportRequest {
    input_path: PathBuf,
    output_path: PathBuf,
    block_frames: u32,
}

impl ExportRequest {
    fn parse(args: &[String]) -> Result<Self, AudioExportError> {
        if args.len() != 4 || args[0] != "--export-wav" {
            return Err(AudioExportError::Usage);
        }
        let block_frames = args[3]
            .parse()
            .map_err(|_| AudioExportError::InvalidBlockFrames(args[3].clone()))?;
        Ok(Self {
            input_path: PathBuf::from(&args[1]),
            output_path: PathBuf::from(&args[2]),
            block_frames,
        })
    }
}

fn read_input(path: &Path) -> Result<Vec<u8>, AudioExportError> {
    let file = File::open(path).map_err(|source| AudioExportError::InputOpen {
        path: path.to_path_buf(),
        source,
    })?;
    let mut reader = file.take((MAX_INPUT_BYTES + 1) as u64);
    let mut bytes = Vec::new();
    bytes
        .try_reserve_exact(MAX_INPUT_BYTES + 1)
        .map_err(|_| AudioExportError::InputAllocationFailed(path.to_path_buf()))?;
    reader
        .read_to_end(&mut bytes)
        .map_err(|source| AudioExportError::InputRead {
            path: path.to_path_buf(),
            source,
        })?;
    if bytes.len() > MAX_INPUT_BYTES {
        return Err(AudioExportError::InputTooLarge {
            path: path.to_path_buf(),
        });
    }
    Ok(bytes)
}

#[derive(Debug)]
enum AudioExportError {
    Usage,
    InvalidBlockFrames(String),
    InputOpen { path: PathBuf, source: io::Error },
    InputRead { path: PathBuf, source: io::Error },
    InputAllocationFailed(PathBuf),
    InputTooLarge { path: PathBuf },
    Render(engine_audio::offline::WavOfflineError),
    Output(audio_output::WaveFileError),
}

impl fmt::Display for AudioExportError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Usage => formatter
                .write_str("usage: openunity-cli --export-wav INPUT.wav OUTPUT.wav BLOCK_FRAMES"),
            Self::InvalidBlockFrames(value) => {
                write!(
                    formatter,
                    "invalid BLOCK_FRAMES value {value:?}; expected u32"
                )
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
            Self::InputTooLarge { path } => write!(
                formatter,
                "input {} exceeds the 16 MiB WAVE import limit",
                path.display()
            ),
            Self::Render(error) => write!(formatter, "WAVE render failed: {error:?}"),
            Self::Output(error) => write!(formatter, "WAVE export failed: {error}"),
        }
    }
}

impl Error for AudioExportError {}
