//! Independent motion failures, separate from concrete native ABI statuses.

use std::fmt;

use engine_math::MathError;

/// Failures of the bounded collider-free motion contract.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PhysicsError {
    /// A number is nonfinite, violates a positive-value rule, or overflows.
    Numeric,
    /// A finite profile value requires behavior outside this backend's scope.
    Unsupported,
    /// The requested identity is absent from this world's body slot.
    InvalidBody,
    /// The body exists but is in the wrong reservation/publication state.
    InvalidState,
    /// This world already owns its one reserved or committed body.
    Capacity,
    /// A private mutation revision cannot advance without wrapping.
    RevisionExhausted,
    /// A prepared step belongs to another world or predates a mutation.
    StalePlan,
}

impl From<MathError> for PhysicsError {
    fn from(error: MathError) -> Self {
        match error {
            MathError::Numeric => Self::Numeric,
            MathError::Unsupported => Self::Unsupported,
        }
    }
}

impl fmt::Display for PhysicsError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Numeric => "invalid physics number or finite result overflow",
            Self::Unsupported => "unsupported collider-free physics profile",
            Self::InvalidBody => "body identity is not owned by this world",
            Self::InvalidState => "body publication state does not permit this operation",
            Self::Capacity => "the one-body world is occupied",
            Self::RevisionExhausted => "physics mutation revision exhausted",
            Self::StalePlan => "physics step plan is stale or belongs to another world",
        })
    }
}

impl std::error::Error for PhysicsError {}
