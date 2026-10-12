use crate::{Guid, InvalidGuid};
use std::collections::BTreeSet;
use std::error::Error;
use std::fmt;
use std::io;
use std::path::PathBuf;

/// Maximum metadata input and maximum bytes read from one metadata file.
pub const MAX_METADATA_BYTES: usize = 1024 * 1024;

/// The supported, validated part of a metadata file.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Metadata {
    /// Asset identity supplied by the metadata header.
    pub guid: Guid,
}

/// A missing, unsupported, malformed, or inaccessible metadata header.
#[derive(Debug)]
pub enum MetadataError {
    /// The metadata file or one of its required ancestors does not exist.
    MissingMetadata(PathBuf),
    /// Filesystem access failed.
    Io {
        /// Path involved in the failed operation.
        path: PathBuf,
        /// Original filesystem error.
        source: io::Error,
    },
    /// A file or ancestor is a symbolic link.
    Symlink(PathBuf),
    /// The supplied path is empty or contains parent traversal.
    InvalidPath(PathBuf),
    /// The supplied metadata path is not a regular file.
    NotRegularFile(PathBuf),
    /// Input exceeds [`MAX_METADATA_BYTES`].
    TooLarge,
    /// Input is not UTF-8.
    InvalidUtf8(std::str::Utf8Error),
    /// A required top-level header was absent.
    MissingHeader(&'static str),
    /// A top-level header key appeared more than once.
    DuplicateHeader(String),
    /// The declared version is not the supported literal `2`.
    UnsupportedVersion(String),
    /// A top-level line has no key/value separator or uses unsupported line endings.
    MalformedHeader {
        /// One-based input line number.
        line: usize,
    },
    /// The GUID value is malformed.
    InvalidGuid(InvalidGuid),
}

impl fmt::Display for MetadataError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingMetadata(path) => write!(f, "missing metadata: {}", path.display()),
            Self::Io { path, source } => write!(f, "cannot read {}: {source}", path.display()),
            Self::Symlink(path) => write!(f, "metadata path is a symlink: {}", path.display()),
            Self::InvalidPath(path) => write!(f, "invalid metadata path: {}", path.display()),
            Self::NotRegularFile(path) => {
                write!(f, "metadata is not a regular file: {}", path.display())
            }
            Self::TooLarge => write!(f, "metadata exceeds {MAX_METADATA_BYTES} bytes"),
            Self::InvalidUtf8(source) => write!(f, "metadata is not UTF-8: {source}"),
            Self::MissingHeader(key) => write!(f, "metadata is missing {key}"),
            Self::DuplicateHeader(key) => write!(f, "duplicate metadata header: {key}"),
            Self::UnsupportedVersion(version) => {
                write!(f, "unsupported metadata fileFormatVersion: {version}")
            }
            Self::MalformedHeader { line } => write!(f, "malformed metadata header at line {line}"),
            Self::InvalidGuid(source) => source.fmt(f),
        }
    }
}

impl Error for MetadataError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Io { source, .. } => Some(source),
            Self::InvalidUtf8(source) => Some(source),
            Self::InvalidGuid(source) => Some(source),
            _ => None,
        }
    }
}

/// Reads the supported UTF-8 header dialect from caller-supplied bytes.
///
/// An optional initial BOM and LF/CRLF line endings are accepted. Exactly one
/// top-level `fileFormatVersion: 2` and `guid` are required. Indented importer
/// lines and other top-level values are not interpreted. No complete YAML or
/// Unity serialization support is implied.
/// Top-level keys are compared case-sensitively after trimming trailing ASCII
/// spaces and tabs. Every repeated top-level key is rejected. Required values
/// are validated at their first occurrence, before examining later headers.
///
/// # Errors
/// Returns a structured error for oversized, non-UTF-8, missing, duplicated,
/// malformed, or unsupported headers.
pub fn parse(bytes: &[u8]) -> Result<Metadata, MetadataError> {
    if bytes.len() > MAX_METADATA_BYTES {
        return Err(MetadataError::TooLarge);
    }
    let text = std::str::from_utf8(bytes).map_err(MetadataError::InvalidUtf8)?;
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    let mut headers = BTreeSet::new();
    let mut guid = None;
    for (index, line) in text.lines().enumerate() {
        if line.contains('\r') {
            return Err(MetadataError::MalformedHeader { line: index + 1 });
        }
        if line.starts_with([' ', '\t']) || line.is_empty() || line.starts_with('#') {
            continue;
        }
        let (key, value) = line
            .split_once(':')
            .ok_or(MetadataError::MalformedHeader { line: index + 1 })?;
        let key = key.trim_end_matches([' ', '\t']);
        let value = value.trim_matches([' ', '\t']);
        if key.is_empty() {
            return Err(MetadataError::MalformedHeader { line: index + 1 });
        }
        if !headers.insert(key) {
            return Err(MetadataError::DuplicateHeader(key.to_owned()));
        }

        match key {
            "fileFormatVersion" => {
                if value != "2" {
                    return Err(MetadataError::UnsupportedVersion(value.to_owned()));
                }
            }
            "guid" => {
                guid = Some(value.parse().map_err(MetadataError::InvalidGuid)?);
            }
            _ => {}
        }
    }
    if !headers.contains("fileFormatVersion") {
        return Err(MetadataError::MissingHeader("fileFormatVersion"));
    }
    Ok(Metadata {
        guid: guid.ok_or(MetadataError::MissingHeader("guid"))?,
    })
}
