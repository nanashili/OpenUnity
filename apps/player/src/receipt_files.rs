//! Exclusive JSON receipt writing and partial-success cleanup.

use crate::render_output::PublishError;
use serde::Serialize;
use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::Path;

pub(super) fn write_json_exclusive<T: Serialize>(
    directory: &Path,
    name: &str,
    value: &T,
    remove_partial_file: bool,
) -> Result<(), PublishError> {
    let path = directory.join(name);
    let bytes = serde_json::to_vec_pretty(value).map_err(PublishError::Serialization)?;
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)
        .map_err(|source| io_error("exclusively create receipt", source))?;
    let publication = write_receipt_bytes(&mut file, &bytes);
    drop(file);

    match publication {
        Ok(()) => Ok(()),
        Err(primary) if !remove_partial_file => Err(primary),
        Err(primary) => match fs::remove_file(path) {
            Ok(()) => Err(primary),
            Err(source) => Err(PublishError::Compound {
                operation: "remove partial success receipt",
                primary: Box::new(primary),
                secondary: Box::new(io_error("remove partial receipt", source)),
            }),
        },
    }
}

fn write_receipt_bytes(file: &mut std::fs::File, bytes: &[u8]) -> Result<(), PublishError> {
    file.write_all(bytes)
        .map_err(|source| io_error("write receipt", source))?;
    file.flush()
        .map_err(|source| io_error("flush receipt", source))?;
    file.sync_all()
        .map_err(|source| io_error("close receipt", source))?;
    Ok(())
}

pub(super) fn io_error(operation: &str, source: io::Error) -> PublishError {
    PublishError::Io {
        operation: operation.into(),
        source,
    }
}
