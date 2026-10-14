//! Stable ABI status codes and native error/identity translation.

use engine_object::ObjectId;
use engine_runtime::{RuntimeError, RuntimeInputError};

/// Invalid or destroyed runtime context.
pub const INVALID_RUNTIME: i32 = -1;
/// Invalid, stale, or foreign object identity.
pub const INVALID_OBJECT: i32 = -2;
/// Storage poisoned by an internal failure.
pub const STORE_UNAVAILABLE: i32 = -3;
/// Process identity space exhausted without wrapping.
pub const IDENTITY_EXHAUSTED: i32 = -4;
/// Invalid name buffer pointer or byte count.
pub const INVALID_BUFFER: i32 = -5;
/// Name input is not strict UTF-8.
pub const INVALID_UTF8: i32 = -6;
/// Read capacity is smaller than the complete current name.
pub const INSUFFICIENT_CAPACITY: i32 = -7;

/// Invalid typed kind, property, index, or flag.
pub const INVALID_KIND: i32 = -8;
/// Invalid numeric input or result.
pub const INVALID_NUMERIC: i32 = -9;
/// Reparenting would create an ancestor cycle.
pub const HIERARCHY_CYCLE: i32 = -10;
/// Singular inverse or unsupported world-preserving reparent.
pub const UNSUPPORTED_TRANSFORM: i32 = -11;
/// Native count exceeds the signed ABI representation.
pub const COUNT_OVERFLOW: i32 = -12;

/// Invalid lifecycle phase, attachment state, or acknowledgement token.
pub const INVALID_LIFECYCLE: i32 = -13;
/// Gameplay callbacks failed and dispatch cannot resume.
pub const GAMEPLAY_FAULTED: i32 = -14;

/// Invalid, stale, or foreign scene handle.
pub const INVALID_SCENE: i32 = -15;
/// Invalid or unsupported scene plan.
pub const INVALID_SCENE_PLAN: i32 = -16;
/// Busy/terminal transaction, owner thread violation, or forbidden legacy call.
pub const STAGING_BUSY: i32 = -17;
/// Physics profile, capacity, mode, synchronization, or hierarchy is unsupported.
pub const PHYSICS_UNSUPPORTED: i32 = -18;
/// Invalid query geometry, including inverted bounds or a zero-length ray.
pub const INVALID_PHYSICS_QUERY: i32 = -19;
/// Sibling index is outside the direct parent's child range.
pub const INVALID_SIBLING_INDEX: i32 = -20;

pub(crate) fn runtime_error(error: RuntimeError) -> i32 {
    match error {
        RuntimeError::InvalidObject => INVALID_OBJECT,
        RuntimeError::InvalidKind => INVALID_KIND,
        RuntimeError::InvalidSiblingIndex => INVALID_SIBLING_INDEX,
        RuntimeError::Numeric => INVALID_NUMERIC,
        RuntimeError::Cycle => HIERARCHY_CYCLE,
        RuntimeError::Unsupported => UNSUPPORTED_TRANSFORM,
        RuntimeError::CountOverflow => COUNT_OVERFLOW,
        RuntimeError::Lifecycle => INVALID_LIFECYCLE,
        RuntimeError::DispatchFaulted => GAMEPLAY_FAULTED,
        RuntimeError::IdentityExhausted => IDENTITY_EXHAUSTED,
        RuntimeError::InvalidScene => INVALID_SCENE,
        RuntimeError::InvalidScenePlan => INVALID_SCENE_PLAN,
        RuntimeError::StagingBusy => STAGING_BUSY,
        RuntimeError::PhysicsUnsupported => PHYSICS_UNSUPPORTED,
        RuntimeError::InvalidPhysicsQuery => INVALID_PHYSICS_QUERY,
    }
}

pub(crate) fn input_status(result: Result<(), RuntimeInputError>) -> i32 {
    result.map_or_else(
        |error| match error {
            RuntimeInputError::Runtime(error) => runtime_error(error),
            RuntimeInputError::Event(_)
            | RuntimeInputError::Binding(_)
            | RuntimeInputError::RetryablePreparation(_)
            | RuntimeInputError::InvariantFailure(_) => INVALID_LIFECYCLE,
        },
        |()| 0,
    )
}

pub(crate) fn object_id(raw: i64) -> Result<ObjectId, RuntimeError> {
    ObjectId::from_raw(raw).ok_or(RuntimeError::InvalidObject)
}
