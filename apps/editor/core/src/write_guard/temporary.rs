//! Own a staged pathname until replacement consumes it or cleanup removes it.

use super::errors::{FinalizationError, RetainedTemporary};
use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_TEMPORARY: AtomicU64 = AtomicU64::new(0);

pub(super) struct TemporaryFile {
    path: Option<PathBuf>,
}

impl TemporaryFile {
    pub(super) fn create(parent: &Path) -> io::Result<(Self, File)> {
        loop {
            let sequence = NEXT_TEMPORARY.fetch_add(1, Ordering::Relaxed);
            let path = parent.join(format!(".openunity-{}-{sequence}.tmp", std::process::id()));
            match OpenOptions::new().write(true).create_new(true).open(&path) {
                Ok(file) => return Ok((Self { path: Some(path) }, file)),
                Err(cause) if cause.kind() == io::ErrorKind::AlreadyExists => continue,
                Err(cause) => return Err(cause),
            }
        }
    }

    pub(super) fn path(&self) -> &Path {
        self.path
            .as_deref()
            .expect("staged pathname remains owned until publish or cleanup")
    }

    pub(super) fn publish(&mut self, destination: &Path, replace: bool) -> io::Result<()> {
        if replace {
            fs::rename(self.path(), destination)?;
            self.path.take();
            Ok(())
        } else {
            fs::hard_link(self.path(), destination)
        }
    }

    pub(super) fn finish(mut self, result: io::Result<()>) -> io::Result<()> {
        let cleanup = self.cleanup();
        match (result, cleanup) {
            (Ok(()), Ok(())) => Ok(()),
            (Err(failure), Ok(())) => Err(failure),
            (Ok(()), Err(cleanup)) => Err(io::Error::new(
                cleanup.kind(),
                FinalizationError::Committed { cleanup },
            )),
            (Err(failure), Err(cleanup)) => Err(io::Error::new(
                failure.kind(),
                FinalizationError::Uncommitted { failure, cleanup },
            )),
        }
    }

    fn cleanup(&mut self) -> io::Result<()> {
        let Some(path) = self.path.take() else {
            return Ok(());
        };
        // Transfer a failed cleanup path into the error. Drop must not retry it:
        // recovery belongs to the caller and the reported file remains available.
        fs::remove_file(&path)
            .map_err(|cause| io::Error::new(cause.kind(), RetainedTemporary { path, cause }))
    }
}

impl Drop for TemporaryFile {
    fn drop(&mut self) {
        if let Err(failure) = self.cleanup() {
            // Emergency unwind fallback only; diagnostics must not cause a second panic.
            let _ = writeln!(
                std::io::stderr().lock(),
                "OpenUnity temporary cleanup failed: {failure}"
            );
        }
    }
}
