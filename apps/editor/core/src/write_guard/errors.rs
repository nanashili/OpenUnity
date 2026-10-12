//! Public editor diagnostics for rejected or failed guarded writes.

use std::path::{Path, PathBuf};
use std::{fmt, io};

/// A filesystem or ownership failure suitable for an editor diagnostic.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WriteError {
    /// The path whose operation was rejected or failed.
    pub path: PathBuf,
    /// Failure detail, including any committed destination or retained temporary file.
    pub message: String,
}

impl fmt::Display for WriteError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}: {}", self.path.display(), self.message)
    }
}

impl std::error::Error for WriteError {}

pub(super) fn error(path: &Path, message: impl fmt::Display) -> WriteError {
    WriteError {
        path: path.to_path_buf(),
        message: message.to_string(),
    }
}

/// A cleanup error owns the retained pathname and its original filesystem error.
#[derive(Debug)]
pub(super) struct RetainedTemporary {
    pub(super) path: PathBuf,
    pub(super) cause: io::Error,
}

impl fmt::Display for RetainedTemporary {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{}; temporary retained at {}",
            self.cause,
            self.path.display()
        )
    }
}

impl std::error::Error for RetainedTemporary {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.cause)
    }
}

#[derive(Debug)]
pub(super) enum FinalizationError {
    Uncommitted {
        failure: io::Error,
        cleanup: io::Error,
    },
    Committed {
        cleanup: io::Error,
    },
}

impl fmt::Display for FinalizationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Uncommitted { failure, cleanup } => {
                write!(
                    formatter,
                    "{failure}; temporary cleanup also failed: {cleanup}"
                )
            }
            Self::Committed { cleanup } => {
                write!(
                    formatter,
                    "destination committed; temporary cleanup failed: {cleanup}"
                )
            }
        }
    }
}

impl std::error::Error for FinalizationError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Uncommitted { failure, .. } => Some(failure),
            Self::Committed { cleanup } => Some(cleanup),
        }
    }
}
