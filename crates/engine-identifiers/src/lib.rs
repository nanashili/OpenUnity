//! Process-local typed identity allocation, independent of object storage.
#![forbid(unsafe_code)]

mod identity;

pub use identity::{IdentityExhausted, ObjectId, allocate_next};
