use std::fmt;
use std::io;
use std::path::PathBuf;

#[derive(Debug)]
pub(in crate::commands::audio_export) enum WaveFileError {
    InvalidDestination(PathBuf),
    Parent {
        path: PathBuf,
        source: io::Error,
    },
    ParentNotDirectory(PathBuf),
    TemporaryCreate {
        path: PathBuf,
        source: io::Error,
    },
    TemporaryCollision(PathBuf),
    SizeOverflow,
    FieldOverflow,
    Write {
        source: io::Error,
    },
    Sync {
        path: PathBuf,
        source: io::Error,
    },
    WriteCleanup {
        path: PathBuf,
        error: Box<Self>,
        cleanup: io::Error,
    },
    Publication {
        temporary_path: PathBuf,
        destination: PathBuf,
        source: io::Error,
    },
    PublishedCleanup {
        temporary_path: PathBuf,
        destination: PathBuf,
        source: io::Error,
    },
}

impl fmt::Display for WaveFileError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidDestination(path) => {
                write!(formatter, "invalid destination {}", path.display())
            }
            Self::Parent { path, source } => write!(
                formatter,
                "cannot inspect destination parent {}: {source}",
                path.display()
            ),
            Self::ParentNotDirectory(path) => write!(
                formatter,
                "destination parent {} is not a directory",
                path.display()
            ),
            Self::TemporaryCreate { path, source } => write!(
                formatter,
                "cannot create temporary export {}: {source}",
                path.display()
            ),
            Self::TemporaryCollision(path) => write!(
                formatter,
                "could not reserve a temporary export name in {}",
                path.display()
            ),
            Self::SizeOverflow => formatter.write_str("WAVE size arithmetic overflowed"),
            Self::FieldOverflow => formatter.write_str("WAVE size does not fit its 32-bit field"),
            Self::Write { source } => write!(formatter, "cannot write WAVE export: {source}"),
            Self::Sync { path, source } => write!(
                formatter,
                "cannot sync temporary export {}: {source}",
                path.display()
            ),
            Self::WriteCleanup {
                path,
                error,
                cleanup,
            } => write!(
                formatter,
                "{}; cleanup of {} also failed: {cleanup}",
                error,
                path.display()
            ),
            Self::Publication {
                temporary_path,
                destination,
                source,
            } => write!(
                formatter,
                "cannot publish {} as {}: {source}; recovery file retained at {}",
                temporary_path.display(),
                destination.display(),
                temporary_path.display()
            ),
            Self::PublishedCleanup {
                temporary_path,
                destination,
                source,
            } => write!(
                formatter,
                "published {} but could not remove temporary name {}: {source}",
                destination.display(),
                temporary_path.display()
            ),
        }
    }
}

impl std::error::Error for WaveFileError {}
