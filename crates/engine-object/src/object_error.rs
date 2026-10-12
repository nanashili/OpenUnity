//! Object-domain validation failures, independent of runtime and ABI statuses.

/// Failure to resolve or mutate an owned object relationship.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ObjectError {
    /// The identity is absent, stale, or owned by another store.
    InvalidObject,
    /// The live identity has a different category from the requested operation.
    InvalidKind,
    /// The process-wide identity allocator is exhausted.
    IdentityExhausted,
    /// Object metadata could not reserve its required storage.
    CapacityExhausted,
    /// Component publication or retirement would violate ownership.
    ComponentState,
}

impl From<crate::IdentityExhausted> for ObjectError {
    fn from(_: crate::IdentityExhausted) -> Self {
        Self::IdentityExhausted
    }
}
