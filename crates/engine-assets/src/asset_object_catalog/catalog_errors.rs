use super::reference::{AssetObjectClass, AssetObjectReference, AssetObjectTarget};
use std::{error::Error, fmt};

/// A complete asset-object catalog could not be constructed.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AssetObjectCatalogBuildError {
    /// A binding refers to a GUID absent from the supplied metadata snapshot.
    MissingAssetGuid {
        /// The rejected composite key.
        reference: AssetObjectReference,
    },
    /// A binding selects a resource absent from the supplied render snapshot.
    MissingRenderTarget {
        /// The key whose binding selected the missing resource.
        reference: AssetObjectReference,
        /// The rejected typed resource target.
        target: AssetObjectTarget,
    },
    /// More than one binding uses the same GUID and local-file-ID pair.
    DuplicateReference {
        /// The conflicting composite key.
        reference: AssetObjectReference,
    },
    /// The binding iterator yielded more entries than the catalog permits.
    BindingLimitExceeded {
        /// Maximum number of accepted bindings.
        max_bindings: usize,
    },
}

impl fmt::Display for AssetObjectCatalogBuildError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingAssetGuid { reference } => write!(
                formatter,
                "asset-object binding references unindexed GUID {}",
                reference.guid()
            ),
            Self::MissingRenderTarget { reference, target } => write!(
                formatter,
                "asset-object binding {reference:?} selects missing render target {target:?}"
            ),
            Self::DuplicateReference { reference } => {
                write!(formatter, "duplicate asset-object binding {reference:?}")
            }
            Self::BindingLimitExceeded { max_bindings } => write!(
                formatter,
                "asset-object binding count exceeds limit {max_bindings}"
            ),
        }
    }
}

impl Error for AssetObjectCatalogBuildError {}

/// A valid asset-object key did not resolve to the requested class.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AssetObjectLookupError {
    /// The reference GUID is absent from this catalog's metadata snapshot.
    MissingAssetGuid {
        /// The unresolved composite key.
        reference: AssetObjectReference,
    },
    /// The GUID is indexed, but this local file ID has no binding.
    MissingLocalFileId {
        /// The unresolved composite key.
        reference: AssetObjectReference,
    },
    /// The key resolves, but its target class differs from the requested class.
    WrongAssetClass {
        /// The key that resolved to a different class.
        reference: AssetObjectReference,
        /// Class requested by the caller.
        expected: AssetObjectClass,
        /// Class selected by the catalog binding.
        actual: AssetObjectClass,
    },
}

impl fmt::Display for AssetObjectLookupError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingAssetGuid { reference } => write!(
                formatter,
                "asset-object GUID {} is not indexed",
                reference.guid()
            ),
            Self::MissingLocalFileId { reference } => {
                write!(formatter, "unbound asset-object reference {reference:?}")
            }
            Self::WrongAssetClass {
                reference,
                expected,
                actual,
            } => write!(
                formatter,
                "reference {reference:?} resolves to {actual}, expected {expected}"
            ),
        }
    }
}

impl Error for AssetObjectLookupError {}
