//! Bounded player-side publication of completed renderer readbacks.
//!
//! This module consumes completed images from `engine_render_api`.
//! It owns output admission and artifact publication; route orchestration and
//! session cleanup remain outside this module.

#[path = "frame_artifacts.rs"]
mod frame_artifacts;
#[path = "output_paths.rs"]
mod output_paths;
#[path = "receipt_files.rs"]
mod receipt_files;
#[path = "receipts.rs"]
mod receipts;

use engine_render_api::{AdapterPolicy, RenderedImage, TargetSize};
use serde::Serialize;
use std::io;
use std::path::{Path, PathBuf};

use receipts::{FailureReceipt, FrameRecord, Receipt, frame_record};

/// Result of the managed/native callback associated with one completed frame.
#[derive(Clone, Debug, Serialize)]
pub enum CallbackResult {
    /// The callback completed successfully and returned this native value.
    ///
    /// `tick_input` records the signed CLI input passed to the route's tick
    /// call. It does not claim that a managed MonoBehaviour observes it.
    Succeeded {
        tick_input: Option<i64>,
        returned: i64,
    },
}

/// One independently attempted cleanup action.
#[derive(Clone, Debug, Serialize)]
pub enum CleanupResult {
    /// The action completed successfully.
    Succeeded,
    /// The action failed without suppressing other cleanup attempts.
    Failed { diagnostic: String },
    /// The caller did not have this action to perform.
    NotAttempted,
}

/// Cleanup facts supplied after renderer/session teardown attempts.
#[derive(Clone, Debug, Serialize)]
pub struct CleanupOutcome {
    /// Result of the session shutdown callback.
    pub shutdown: CleanupResult,
    /// Result of the independent native session close.
    pub close: CleanupResult,
    /// Result of the renderer/device close or drop attempt.
    pub renderer: CleanupResult,
}

#[derive(Debug)]
pub enum PublishError {
    /// The output token or an existing path component violates admission.
    InvalidOutputPath(String),
    /// A frame or backend receipt violates the requested output contract.
    InvalidFrame(String),
    /// An artifact operation failed.
    Io {
        operation: String,
        source: io::Error,
    },
    /// JSON serialization failed.
    Serialization(serde_json::Error),
    /// Render execution failed after output admission.
    Execution(String),
    /// The primary publication and its follow-up both failed.
    Compound {
        operation: &'static str,
        primary: Box<Self>,
        secondary: Box<Self>,
    },
    /// The original failure and failure-receipt failure both remain visible.
    FailureReceipt {
        primary: Box<Self>,
        receipt: Box<Self>,
    },
}

impl std::fmt::Display for PublishError {
    fn fmt(&self, output: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidOutputPath(message) => write!(output, "invalid output path: {message}"),
            Self::InvalidFrame(message) => write!(output, "invalid rendered frame: {message}"),
            Self::Io { operation, source } => write!(output, "{operation}: {source}"),
            Self::Serialization(error) => write!(output, "receipt serialization: {error}"),
            Self::Execution(message) => write!(output, "render execution: {message}"),
            Self::Compound {
                operation,
                primary,
                secondary,
            } => write!(
                output,
                "{operation} failed ({primary}); follow-up failed ({secondary})"
            ),
            Self::FailureReceipt { primary, receipt } => {
                write!(
                    output,
                    "publication failed ({primary}); failure receipt failed ({receipt})"
                )
            }
        }
    }
}

impl std::error::Error for PublishError {}

/// A publisher owning one freshly admitted output directory.
pub struct RenderOutputPublisher {
    directory: PathBuf,
    requested_size: TargetSize,
    requested_policy: AdapterPolicy,
    frames: Vec<FrameRecord>,
}

impl RenderOutputPublisher {
    /// Admit and exclusively create an output directory.
    pub fn create(
        output: impl AsRef<Path>,
        requested_size: TargetSize,
        requested_policy: AdapterPolicy,
    ) -> Result<Self, PublishError> {
        let directory = output_paths::create_output_directory(output.as_ref())?;
        Ok(Self {
            directory,
            requested_size,
            requested_policy,
            frames: Vec::new(),
        })
    }

    /// Publish one completed readback and retain its record only after close.
    pub fn publish_frame(
        &mut self,
        ordinal: u64,
        image: &RenderedImage,
        callback: CallbackResult,
    ) -> Result<(), PublishError> {
        self.require_next_ordinal(ordinal)?;
        let artifact = frame_artifacts::publish(
            &self.directory,
            ordinal,
            image,
            self.requested_size,
            self.requested_policy,
        )?;
        self.frames
            .push(frame_record(ordinal, artifact, image, callback));
        Ok(())
    }

    /// Write the success receipt after the caller has completed cleanup/drop.
    pub fn finish(self, cleanup: CleanupOutcome) -> Result<(), PublishError> {
        let receipt = Receipt::success(self, cleanup);
        match receipt_files::write_json_exclusive(
            &receipt.directory,
            "receipt.json",
            &receipt,
            true,
        ) {
            Ok(()) => Ok(()),
            Err(primary) => {
                let failure = FailureReceipt::from_success(receipt, primary.to_string());
                match receipt_files::write_json_exclusive(
                    &failure.directory,
                    "failure.json",
                    &failure,
                    false,
                ) {
                    Ok(()) => Err(primary),
                    Err(receipt) => Err(PublishError::FailureReceipt {
                        primary: Box::new(primary),
                        receipt: Box::new(receipt),
                    }),
                }
            }
        }
    }

    /// Write a failure receipt while retaining completed-frame evidence.
    pub fn finish_failure(
        self,
        primary: PublishError,
        cleanup: CleanupOutcome,
    ) -> Result<(), PublishError> {
        let receipt = FailureReceipt::new(self, primary.to_string(), cleanup);
        match receipt_files::write_json_exclusive(
            &receipt.directory,
            "failure.json",
            &receipt,
            false,
        ) {
            Ok(()) => Ok(()),
            Err(receipt_error) => Err(PublishError::FailureReceipt {
                primary: Box::new(primary),
                receipt: Box::new(receipt_error),
            }),
        }
    }

    fn require_next_ordinal(&self, ordinal: u64) -> Result<(), PublishError> {
        if ordinal != self.frames.len() as u64 {
            return Err(PublishError::InvalidFrame("non-contiguous ordinal".into()));
        }
        Ok(())
    }
}
