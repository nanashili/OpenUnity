//! Bounded persistence for independent OpenUnity scene documents.

mod depth;
mod error;
mod filesystem;
mod wire;

pub use error::SceneIoError;
pub use filesystem::{load, save};
pub use wire::{decode, encode};

#[cfg(test)]
mod tests;
