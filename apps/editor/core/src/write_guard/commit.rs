//! Write, sync and close staged output before publishing its directory entry.

use super::temporary::TemporaryFile;
use std::fs::File;
use std::io::{self, Write};
use std::path::Path;

pub(super) fn atomic_commit(destination: &Path, bytes: &[u8], replace: bool) -> io::Result<()> {
    let parent = destination
        .parent()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "save target has no parent"))?;
    let (mut temporary, file) = TemporaryFile::create(parent)?;
    let result = write_and_publish(file, &mut temporary, destination, bytes, replace);
    temporary.finish(result)
}

fn write_and_publish(
    mut file: File,
    temporary: &mut TemporaryFile,
    destination: &Path,
    bytes: &[u8],
    replace: bool,
) -> io::Result<()> {
    file.write_all(bytes)?;
    file.sync_all()?;
    drop(file);

    temporary.publish(destination, replace)
}
