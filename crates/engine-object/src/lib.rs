//! Owned identities, names, and component relationships for reviewed object contracts.
#![forbid(unsafe_code)]

mod component_ownership;
mod component_publication;
mod object_access;
mod object_error;
mod object_retirement;
mod object_store;
mod registry;

pub use component_ownership::PreparedRigidbodyReservation;
pub use component_publication::{
    PreparedObjectRelease, PreparedObjectRetirement, PreparedRigidbodyPublication,
};
pub use engine_identifiers::{IdentityExhausted, ObjectId};
pub use object_access::MAX_OBJECT_NAME_BYTES;
pub use object_error::ObjectError;
pub use object_store::{ObjectKind, ObjectStore, PreparedGameObject};
pub use registry::Registry;

#[cfg(test)]
mod tests;
