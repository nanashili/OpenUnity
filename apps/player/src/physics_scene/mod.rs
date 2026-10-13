//! Bounded native collision-scene rendering for player acceptance runs.

pub(crate) mod arguments;

mod error;
mod execution;
mod geometry;
mod receipt;
mod rendering;
mod scene;
mod simulation;
mod snapshot;

pub(crate) use execution::run;
