//! Deterministic metadata discovery for directly embedded project packages.

mod candidates;
mod discovery;
mod manifest;

pub(in crate::project) use discovery::discover_embedded_packages;
