use std::error::Error;
use std::fmt;

/// A host setup, loading, ABI, or callback failure with operation context.
#[derive(Debug)]
pub struct HostError {
    operation: &'static str,
    detail: String,
    status: Option<i32>,
}

impl HostError {
    pub(super) fn setup(operation: &'static str, detail: impl ToString) -> Self {
        Self {
            operation,
            detail: detail.to_string(),
            status: None,
        }
    }

    pub(super) fn status(operation: &'static str, status: i32) -> Self {
        Self {
            operation,
            detail: format!("status {status} (0x{:08x})", status as u32),
            status: Some(status),
        }
    }

    pub(super) fn with_cleanup(mut self, cleanup: HostError) -> Self {
        self.detail = format!("{}; cleanup also failed: {cleanup}", self.detail);
        self
    }

    /// Return the native/managed status, if this was an ABI failure.
    pub fn code(&self) -> Option<i32> {
        self.status
    }
}

impl fmt::Display for HostError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.operation, self.detail)
    }
}

impl Error for HostError {}
