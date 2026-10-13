use crate::AssetError;
use std::error::Error;
use std::fmt;

/// Deterministic failures for the bounded triangle-only OBJ subset.
#[non_exhaustive]
#[derive(Debug)]
pub enum ObjImportError {
    /// The input is larger than the importer byte limit.
    InputTooLarge {
        /// Zero-based offset of the first byte beyond the importer byte limit.
        byte_offset: usize,
    },
    /// The first non-ASCII byte was found at the given zero-based offset.
    NonAscii {
        /// Zero-based offset of the first non-ASCII byte.
        byte_offset: usize,
    },
    /// A forbidden text-control byte was found at the given zero-based offset.
    InvalidTextByte {
        /// Zero-based offset of the forbidden text-control byte.
        byte_offset: usize,
    },
    /// A statement outside the supported `v` and triangular `f` subset was found.
    UnsupportedStatement {
        /// One-based OBJ input line number.
        line: usize,
    },
    /// A vertex statement does not contain exactly three coordinates.
    MalformedVertex {
        /// One-based OBJ input line number.
        line: usize,
    },
    /// A vertex coordinate is malformed, non-finite, or outside the supported range.
    InvalidCoordinate {
        /// One-based OBJ input line number.
        line: usize,
    },
    /// A face does not contain exactly three vertex references.
    UnsupportedFaceArity {
        /// One-based OBJ input line number.
        line: usize,
    },
    /// A face uses a reference form outside positive one-based vertex indices.
    UnsupportedFaceReference {
        /// One-based OBJ input line number.
        line: usize,
    },
    /// A positive one-based vertex index is zero, malformed, forward, or out of range.
    InvalidFaceReference {
        /// One-based OBJ input line number.
        line: usize,
    },
    /// The mesh position limit would be exceeded by this vertex.
    PositionLimitExceeded {
        /// One-based OBJ input line number.
        line: usize,
    },
    /// The mesh index limit would be exceeded by this face.
    IndexLimitExceeded {
        /// One-based OBJ input line number.
        line: usize,
    },
    /// The validated mesh constructor rejected the parsed geometry.
    Mesh(AssetError),
}

impl fmt::Display for ObjImportError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InputTooLarge { byte_offset } => write!(
                formatter,
                "OBJ input exceeds the byte limit at offset {byte_offset}"
            ),
            Self::NonAscii { byte_offset } => {
                write!(
                    formatter,
                    "OBJ input contains non-ASCII byte at offset {byte_offset}"
                )
            }
            Self::InvalidTextByte { byte_offset } => {
                write!(
                    formatter,
                    "OBJ input contains invalid text byte at offset {byte_offset}"
                )
            }
            Self::UnsupportedStatement { line } => {
                write!(formatter, "unsupported OBJ statement at line {line}")
            }
            Self::MalformedVertex { line } => {
                write!(formatter, "malformed OBJ vertex at line {line}")
            }
            Self::InvalidCoordinate { line } => {
                write!(formatter, "invalid OBJ coordinate at line {line}")
            }
            Self::UnsupportedFaceArity { line } => {
                write!(formatter, "unsupported OBJ face arity at line {line}")
            }
            Self::UnsupportedFaceReference { line } => {
                write!(formatter, "unsupported OBJ face reference at line {line}")
            }
            Self::InvalidFaceReference { line } => {
                write!(formatter, "invalid OBJ face reference at line {line}")
            }
            Self::PositionLimitExceeded { line } => {
                write!(formatter, "OBJ position limit exceeded at line {line}")
            }
            Self::IndexLimitExceeded { line } => {
                write!(formatter, "OBJ index limit exceeded at line {line}")
            }
            Self::Mesh(source) => source.fmt(formatter),
        }
    }
}

impl Error for ObjImportError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Mesh(source) => Some(source),
            _ => None,
        }
    }
}

impl From<AssetError> for ObjImportError {
    fn from(source: AssetError) -> Self {
        Self::Mesh(source)
    }
}
