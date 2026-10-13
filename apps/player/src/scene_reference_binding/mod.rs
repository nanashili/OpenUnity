//! Player-owned composition of extracted scene references and catalog targets.

mod binding_rows;
mod errors;
mod pairing;
mod pointer_resolution;
mod types;

pub use pointer_resolution::resolve_scene_bindings;
pub use types::{AssetBindingRow, BindingKind, PointerResolutionError, ResolvedSceneBindings};
