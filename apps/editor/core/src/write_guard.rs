//! Filesystem guards for independently owned editor documents.
//!
//! Callers supply every protected source-project, installation and research root.
//! Concurrent hostile filesystem mutation is outside this policy's guarantee.

mod commit;
mod errors;
mod ownership;
mod paths;
mod policy;
mod temporary;

pub use errors::WriteError;
pub use ownership::MAX_EXISTING_BYTES;
pub use policy::WritePolicy;

#[cfg(test)]
mod temporary_tests;
#[cfg(test)]
mod tests;
