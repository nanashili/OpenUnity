//! Bounded native collision-scene rendering for player acceptance runs.

pub(crate) mod arguments;

pub(crate) mod error;
pub(crate) mod execution;
pub(crate) mod geometry;
mod receipt;
pub(crate) mod rendering;
pub(crate) mod scene;
pub(crate) mod simulation;
pub(crate) mod snapshot;

pub(crate) use execution::run;
