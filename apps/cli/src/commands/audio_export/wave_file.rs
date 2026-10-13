use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use engine_audio::offline::OfflineOutput;

mod error;
pub(super) use error::WaveFileError;

const TEMP_ATTEMPTS: u32 = 32;
const WRITE_BUFFER_BYTES: usize = 4096;

pub(super) fn publish(
    destination: &Path,
    output: &Arc<OfflineOutput>,
) -> Result<(), WaveFileError> {
    let layout = OutputLayout::from_output(output)?;
    let parent = destination
        .parent()
        .filter(|path| !path.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    ensure_parent_directory(parent)?;
    let (temporary_path, mut file) = create_temporary(parent, destination)?;

    if let Err(error) = write_complete_file(&mut file, output, layout) {
        drop(file);
        return Err(cleanup_write_failure(temporary_path, error));
    }
    if let Err(error) = file.sync_all() {
        drop(file);
        let sync_error = WaveFileError::Sync {
            path: temporary_path.clone(),
            source: error,
        };
        return Err(cleanup_write_failure(temporary_path, sync_error));
    }
    drop(file);

    if let Err(source) = fs::hard_link(&temporary_path, destination) {
        return Err(WaveFileError::Publication {
            temporary_path,
            destination: destination.to_path_buf(),
            source,
        });
    }
    if let Err(source) = fs::remove_file(&temporary_path) {
        return Err(WaveFileError::PublishedCleanup {
            temporary_path,
            destination: destination.to_path_buf(),
            source,
        });
    }
    Ok(())
}

fn ensure_parent_directory(parent: &Path) -> Result<(), WaveFileError> {
    let metadata = fs::metadata(parent).map_err(|source| WaveFileError::Parent {
        path: parent.to_path_buf(),
        source,
    })?;
    if !metadata.is_dir() {
        return Err(WaveFileError::ParentNotDirectory(parent.to_path_buf()));
    }
    Ok(())
}

fn create_temporary(parent: &Path, destination: &Path) -> Result<(PathBuf, File), WaveFileError> {
    if destination.file_name().is_none() {
        return Err(WaveFileError::InvalidDestination(destination.to_path_buf()));
    }

    for attempt in 0..TEMP_ATTEMPTS {
        let temporary_path = parent.join(format!(
            ".openunity-export-{}-{attempt}.tmp",
            std::process::id()
        ));
        match OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary_path)
        {
            Ok(file) => return Ok((temporary_path, file)),
            Err(source) if source.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(source) => {
                return Err(WaveFileError::TemporaryCreate {
                    path: temporary_path,
                    source,
                });
            }
        }
    }
    Err(WaveFileError::TemporaryCollision(parent.to_path_buf()))
}

fn write_complete_file(
    file: &mut File,
    output: &OfflineOutput,
    layout: OutputLayout,
) -> Result<(), WaveFileError> {
    file.write_all(&layout.header)
        .map_err(|source| WaveFileError::Write { source })?;
    let mut buffer = [0_u8; WRITE_BUFFER_BYTES];
    let mut used = 0usize;
    for sample in output.samples() {
        let encoded = sample.to_le_bytes();
        if used + encoded.len() > buffer.len() {
            file.write_all(&buffer[..used])
                .map_err(|source| WaveFileError::Write { source })?;
            used = 0;
        }
        buffer[used..used + encoded.len()].copy_from_slice(&encoded);
        used += encoded.len();
    }
    if used > 0 {
        file.write_all(&buffer[..used])
            .map_err(|source| WaveFileError::Write { source })?;
    }
    file.flush()
        .map_err(|source| WaveFileError::Write { source })?;
    Ok(())
}

fn cleanup_write_failure(path: PathBuf, error: WaveFileError) -> WaveFileError {
    match fs::remove_file(&path) {
        Ok(()) => error,
        Err(cleanup) => WaveFileError::WriteCleanup {
            path,
            error: Box::new(error),
            cleanup,
        },
    }
}

struct OutputLayout {
    header: [u8; 44],
}

impl OutputLayout {
    fn from_output(output: &OfflineOutput) -> Result<Self, WaveFileError> {
        let channels = u32::from(output.channels().get());
        let sample_rate = output.sample_rate_hz().get();
        let sample_count = output.samples().len();
        let data_bytes = sample_count
            .checked_mul(4)
            .ok_or(WaveFileError::SizeOverflow)?;
        let data_bytes_u32 = u32::try_from(data_bytes).map_err(|_| WaveFileError::FieldOverflow)?;
        let riff_size = 36u32
            .checked_add(data_bytes_u32)
            .ok_or(WaveFileError::FieldOverflow)?;
        let block_align = channels
            .checked_mul(4)
            .ok_or(WaveFileError::FieldOverflow)?;
        let block_align = u16::try_from(block_align).map_err(|_| WaveFileError::FieldOverflow)?;
        let byte_rate = sample_rate
            .checked_mul(u32::from(block_align))
            .ok_or(WaveFileError::FieldOverflow)?;
        let channels = u16::try_from(channels).map_err(|_| WaveFileError::FieldOverflow)?;

        let mut header = [0_u8; 44];
        header[0..4].copy_from_slice(b"RIFF");
        header[4..8].copy_from_slice(&riff_size.to_le_bytes());
        header[8..12].copy_from_slice(b"WAVE");
        header[12..16].copy_from_slice(b"fmt ");
        header[16..20].copy_from_slice(&16u32.to_le_bytes());
        header[20..22].copy_from_slice(&3u16.to_le_bytes());
        header[22..24].copy_from_slice(&channels.to_le_bytes());
        header[24..28].copy_from_slice(&sample_rate.to_le_bytes());
        header[28..32].copy_from_slice(&byte_rate.to_le_bytes());
        header[32..34].copy_from_slice(&block_align.to_le_bytes());
        header[34..36].copy_from_slice(&32u16.to_le_bytes());
        header[36..40].copy_from_slice(b"data");
        header[40..44].copy_from_slice(&data_bytes_u32.to_le_bytes());
        Ok(Self { header })
    }
}
