//! In-memory resolution of explicit typed asset-object references.

mod catalog;
mod catalog_errors;
mod reference;

pub use catalog::AssetObjectCatalog;
pub use catalog_errors::{AssetObjectCatalogBuildError, AssetObjectLookupError};
pub use reference::{
    AssetObjectBinding, AssetObjectClass, AssetObjectReference, AssetObjectTarget,
    InvalidReference, InvalidReferenceComponent,
};
