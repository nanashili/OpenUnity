//! Rust-driven .NET hosting for the reviewed script and gameplay host ABIs.
//!
//! This does not implement Unity APIs. Only the selected hostfxr and reviewed
//! managed bridge may supply native function pointers. CoreCLR and its libraries
//! remain process-resident; closing a script releases its session, not the runtime.

mod abi;
mod error;
mod host;
mod hostfxr;
mod platform;

pub use error::HostError;
pub use host::{ManagedHost, ScriptSession, validate_time_source};
