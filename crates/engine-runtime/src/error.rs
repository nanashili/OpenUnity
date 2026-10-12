//! Failure taxonomy for the independently implemented runtime contracts.

/// Independent typed-extension failures; these are not Unity native statuses.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RuntimeError {
    /// Identity is stale, foreign, or absent.
    InvalidObject,
    /// Object kind, index, or property is unsupported.
    InvalidKind,
    /// A numeric input or result is invalid.
    Numeric,
    /// A parent assignment would form a cycle.
    Cycle,
    /// A singular inverse or unsupported reparent was requested.
    Unsupported,
    /// A collection size cannot be represented by the ABI.
    CountOverflow,
    /// Invalid attachment, lifecycle phase, or callback acknowledgement.
    Lifecycle,
    /// Managed gameplay dispatch failed and cannot continue.
    DispatchFaulted,
    /// Process identities or per-context callback tokens are exhausted.
    IdentityExhausted,
    /// Invalid, stale, or foreign scene handle.
    InvalidScene,
    /// Invalid or unsupported scene plan.
    InvalidScenePlan,
    /// Staging, owner thread, or terminal operation violation.
    StagingBusy,
}

impl From<engine_math::MathError> for RuntimeError {
    fn from(error: engine_math::MathError) -> Self {
        match error {
            engine_math::MathError::Numeric => Self::Numeric,
            engine_math::MathError::Unsupported => Self::Unsupported,
        }
    }
}

impl From<engine_object::ObjectError> for RuntimeError {
    fn from(error: engine_object::ObjectError) -> Self {
        match error {
            engine_object::ObjectError::InvalidObject => Self::InvalidObject,
            engine_object::ObjectError::InvalidKind => Self::InvalidKind,
            engine_object::ObjectError::IdentityExhausted => Self::IdentityExhausted,
            engine_object::ObjectError::ComponentState => Self::Lifecycle,
        }
    }
}
