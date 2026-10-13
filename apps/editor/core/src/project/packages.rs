//! Bounded metadata discovery for embedded project packages.

mod catalog;
mod filesystem;
mod listing;
mod metadata;
mod model;

pub(super) use catalog::discover_embedded_packages;
pub(super) use metadata::is_valid_technical_name;
pub(super) use model::{PackageCatalog, PackageSourceDescriptor};
