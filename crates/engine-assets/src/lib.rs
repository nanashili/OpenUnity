//! Bounded, read-only metadata headers; this is not a YAML parser or asset importer.

mod guid;
mod metadata;
mod metadata_index;
mod metadata_reader;

pub use guid::{Guid, InvalidGuid};
pub use metadata::{MAX_METADATA_BYTES, Metadata, MetadataError, parse};
pub use metadata_index::{DuplicateGuid, MetadataIndex};
pub use metadata_reader::read;

#[cfg(test)]
mod tests;
