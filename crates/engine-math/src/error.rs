//! Numeric primitive failures, independent of runtime ownership.

/// Failures from independently authored numeric operations.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MathError {
    /// A numeric input or result is invalid.
    Numeric,
    /// A singular component division was requested.
    Unsupported,
}
