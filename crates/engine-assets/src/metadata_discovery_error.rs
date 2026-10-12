use crate::{DuplicateGuid, MetadataError};
use std::error::Error;
use std::fmt;
use std::io;
use std::path::PathBuf;

/// Bounds one metadata discovery operation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MetadataDiscoveryLimits {
    /// Maximum directory depth below the supplied root.
    pub max_depth: usize,
    /// Maximum number of entries yielded by all visited directories.
    pub max_entries: usize,
}

/// Failure while building a complete metadata index.
pub enum MetadataDiscoveryError {
    /// The root is not an absolute directory accepted by its path components.
    InvalidRoot(PathBuf),
    /// Filesystem access failed for a particular path.
    Io {
        /// Path involved in the failed filesystem operation.
        path: PathBuf,
        /// Original filesystem error.
        source: io::Error,
    },
    /// A supplied path is empty or contains parent traversal.
    InvalidPath(PathBuf),
    /// A path does not remain lexically under the supplied root.
    OutsideRoot(PathBuf),
    /// A root, directory, metadata file, or derived asset path is a symlink.
    Symlink(PathBuf),
    /// An entry is neither a directory nor a regular file.
    UnsupportedEntryType(PathBuf),
    /// A directory exceeds the configured depth.
    DepthLimit(PathBuf),
    /// The walk yielded more entries than permitted.
    EntryCountLimit {
        /// Directory whose entry caused the limit to be exceeded.
        path: PathBuf,
        /// Configured maximum number of entries.
        max_entries: usize,
    },
    /// The selected metadata file was rejected by the existing reader.
    Metadata {
        /// Metadata file whose contents were rejected.
        path: PathBuf,
        /// Original reader error.
        source: MetadataError,
    },
    /// Two distinct asset paths claim one GUID.
    DuplicateGuid(DuplicateGuid),
}

impl fmt::Debug for MetadataDiscoveryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidRoot(path) => f.debug_tuple("InvalidRoot").field(path).finish(),
            Self::Io { path, source } => f
                .debug_struct("Io")
                .field("path", path)
                .field("source", source)
                .finish(),
            Self::InvalidPath(path) => f.debug_tuple("InvalidPath").field(path).finish(),
            Self::OutsideRoot(path) => f.debug_tuple("OutsideRoot").field(path).finish(),
            Self::Symlink(path) => f.debug_tuple("Symlink").field(path).finish(),
            Self::UnsupportedEntryType(path) => {
                f.debug_tuple("UnsupportedEntryType").field(path).finish()
            }
            Self::DepthLimit(path) => f.debug_tuple("DepthLimit").field(path).finish(),
            Self::EntryCountLimit { path, max_entries } => f
                .debug_struct("EntryCountLimit")
                .field("path", path)
                .field("max_entries", max_entries)
                .finish(),
            Self::Metadata { path, source } => {
                let detail = match source {
                    MetadataError::TooLarge => "too large".to_owned(),
                    _ => format!("{source:?}"),
                };
                f.debug_struct("Metadata")
                    .field("path", path)
                    .field("source", &detail)
                    .finish()
            }
            Self::DuplicateGuid(source) => f
                .debug_struct("DuplicateGuid")
                .field("guid", &source.guid.to_string())
                .field("first_path", &source.first_path)
                .field("duplicate_path", &source.duplicate_path)
                .finish(),
        }
    }
}

impl fmt::Display for MetadataDiscoveryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidRoot(_) => write!(f, "expected absolute project directory"),
            Self::Io { source, .. } => source.fmt(f),
            Self::InvalidPath(_) => write!(f, "invalid project path"),
            Self::OutsideRoot(_) => write!(f, "path outside project root"),
            Self::Symlink(_) => write!(f, "symbolic project path"),
            Self::UnsupportedEntryType(_) => write!(f, "unsupported project file type"),
            Self::DepthLimit(_) => write!(f, "project directory depth limit"),
            Self::EntryCountLimit { .. } => write!(f, "project entry count limit"),
            Self::Metadata { source, .. } => source.fmt(f),
            Self::DuplicateGuid(source) => source.fmt(f),
        }
    }
}

impl Error for MetadataDiscoveryError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Io { source, .. } => Some(source),
            Self::Metadata { source, .. } => Some(source),
            Self::DuplicateGuid(source) => Some(source),
            _ => None,
        }
    }
}
