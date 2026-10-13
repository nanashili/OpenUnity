//! Bounded file reads with metadata checks at configured read boundaries.

use std::fs::File;
use std::io::Read;
use std::path::Path;

pub(in crate::project::packages) enum BoundedReadError {
    LimitExceeded { bytes_read: usize },
    Failed { bytes_read: usize },
}

pub(in crate::project::packages) fn read_bounded_file(
    path: &Path,
    file_limit: usize,
    aggregate_remaining: Option<usize>,
) -> Result<Vec<u8>, BoundedReadError> {
    let file = File::open(path).map_err(|_| BoundedReadError::Failed { bytes_read: 0 })?;
    let metadata = file
        .metadata()
        .map_err(|_| BoundedReadError::Failed { bytes_read: 0 })?;

    if !metadata.is_file() {
        return Err(BoundedReadError::Failed { bytes_read: 0 });
    }
    if metadata.len() > file_limit as u64
        || aggregate_remaining.is_some_and(|remaining| metadata.len() > remaining as u64)
    {
        return Err(BoundedReadError::LimitExceeded { bytes_read: 0 });
    }

    let read_limit = aggregate_remaining.map_or(file_limit, |remaining| file_limit.min(remaining));
    let mut bytes = Vec::with_capacity(metadata.len() as usize);
    let mut bounded_file = file.take(read_limit as u64);

    if bounded_file.read_to_end(&mut bytes).is_err() {
        return Err(BoundedReadError::Failed {
            bytes_read: bytes.len(),
        });
    }

    if bytes.len() > file_limit {
        return Err(BoundedReadError::LimitExceeded {
            bytes_read: bytes.len(),
        });
    }

    match file_grew_past_read_boundary(&bounded_file, &bytes, file_limit, aggregate_remaining) {
        Ok(true) => {
            return Err(BoundedReadError::LimitExceeded {
                bytes_read: bytes.len(),
            });
        }
        Ok(false) => {}
        Err(_) => {
            return Err(BoundedReadError::Failed {
                bytes_read: bytes.len(),
            });
        }
    }

    Ok(bytes)
}

fn file_grew_past_read_boundary(
    bounded_file: &std::io::Take<File>,
    bytes: &[u8],
    file_limit: usize,
    aggregate_remaining: Option<usize>,
) -> std::io::Result<bool> {
    let file_limit_reached = bytes.len() == file_limit;
    let aggregate_limit_reached =
        aggregate_remaining.is_some_and(|remaining| bytes.len() == remaining);
    if !file_limit_reached && !aggregate_limit_reached {
        return Ok(false);
    }

    bounded_file
        .get_ref()
        .metadata()
        .map(|metadata| metadata.len() > bytes.len() as u64)
}
