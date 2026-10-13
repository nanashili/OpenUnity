//! Failure taxonomy for the independently implemented runtime contracts.

use engine_physics_api::{CollisionError, PhysicsError, QueryError};

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
    /// A finite physics operation is outside the bounded native contract.
    PhysicsUnsupported,
    /// The requested query geometry is invalid.
    InvalidPhysicsQuery,
}

impl From<PhysicsError> for RuntimeError {
    fn from(error: PhysicsError) -> Self {
        match error {
            PhysicsError::Numeric => Self::Numeric,
            PhysicsError::Unsupported | PhysicsError::Capacity => Self::PhysicsUnsupported,
            PhysicsError::InvalidBody => Self::InvalidObject,
            PhysicsError::InvalidState | PhysicsError::StalePlan => Self::Lifecycle,
            PhysicsError::RevisionExhausted => Self::CountOverflow,
        }
    }
}

impl From<QueryError> for RuntimeError {
    fn from(error: QueryError) -> Self {
        match error {
            QueryError::InvalidQuery => Self::InvalidPhysicsQuery,
            QueryError::Numeric => Self::Numeric,
            QueryError::Capacity => Self::PhysicsUnsupported,
            QueryError::DuplicateOwner => Self::Lifecycle,
            QueryError::RevisionExhausted => Self::CountOverflow,
        }
    }
}

impl From<CollisionError> for RuntimeError {
    fn from(error: CollisionError) -> Self {
        match error {
            CollisionError::InvalidNumber
            | CollisionError::InvalidDimensions
            | CollisionError::InvalidTimestep => Self::Numeric,
            CollisionError::InvalidBody => Self::InvalidObject,
            CollisionError::BodyAlreadyExists => Self::Lifecycle,
            CollisionError::Capacity => Self::PhysicsUnsupported,
        }
    }
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
            engine_object::ObjectError::CapacityExhausted => Self::CountOverflow,
            engine_object::ObjectError::ComponentState => Self::Lifecycle,
        }
    }
}
