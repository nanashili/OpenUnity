//! Bound and validate existing documents before creating temporary output.

use super::errors::{WriteError, error};
use std::fs::{File, Metadata};
use std::io::Read;
use std::path::Path;

/// Maximum existing document size accepted for ownership validation.
pub const MAX_EXISTING_BYTES: usize = 16 * 1024 * 1024;

pub(super) fn validate_existing(
    supplied: &Path,
    destination: &Path,
    metadata: Option<Metadata>,
    validator: impl FnOnce(&[u8]) -> Result<(), String>,
) -> Result<bool, WriteError> {
    let Some(metadata) = metadata else {
        return Ok(false);
    };
    if !metadata.is_file() {
        return Err(error(
            supplied,
            "existing save target must be a regular file",
        ));
    }
    if metadata.len() > MAX_EXISTING_BYTES as u64 {
        return Err(error(
            supplied,
            "existing save target exceeds the 16 MiB limit",
        ));
    }

    let mut existing = Vec::new();
    File::open(destination)
        .and_then(|file| {
            file.take(MAX_EXISTING_BYTES as u64 + 1)
                .read_to_end(&mut existing)
        })
        .map_err(|cause| error(supplied, cause))?;
    if existing.len() > MAX_EXISTING_BYTES {
        return Err(error(
            supplied,
            "existing save target exceeds the 16 MiB limit",
        ));
    }
    validator(&existing).map_err(|reason| {
        error(
            supplied,
            format!("existing document is not owned: {reason}"),
        )
    })?;
    Ok(true)
}
