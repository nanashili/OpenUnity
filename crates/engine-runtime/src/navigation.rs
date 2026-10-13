//! Runtime-owned movement routes over immutable navigation graphs.

mod advancement;
mod planning;
mod types;

pub use types::{NavigationRoute, RuntimeNavigationError};
