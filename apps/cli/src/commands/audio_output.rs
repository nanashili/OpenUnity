//! Shared no-clobber publication for CLI audio commands.

mod wave_file;

pub(in crate::commands) use wave_file::WaveFileError;

pub(in crate::commands) fn publish(
    destination: &std::path::Path,
    output: &std::sync::Arc<engine_audio::offline::OfflineOutput>,
) -> Result<(), WaveFileError> {
    wave_file::publish(destination, output)
}
