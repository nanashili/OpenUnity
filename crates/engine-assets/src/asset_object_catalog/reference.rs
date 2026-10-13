use crate::Guid;
use std::{error::Error, fmt};

/// A stable class label used in typed asset-object lookup diagnostics.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum AssetObjectClass {
    /// A validated mesh in the catalog's render-resource snapshot.
    Mesh,
    /// A validated unlit material in the catalog's render-resource snapshot.
    UnlitMaterial,
}

impl fmt::Display for AssetObjectClass {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Mesh => f.write_str("mesh"),
            Self::UnlitMaterial => f.write_str("unlit material"),
        }
    }
}

/// The component of an asset-object reference rejected by its constructor.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum InvalidReferenceComponent {
    /// The reference used the all-zero GUID.
    Guid,
    /// The reference used local file ID zero.
    LocalFileId,
}

/// A checked asset-object identity within one catalog snapshot.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct AssetObjectReference {
    guid: Guid,
    local_file_id: i64,
}

impl AssetObjectReference {
    /// Creates a reference from its GUID and signed local file ID.
    ///
    /// Zero local IDs and the all-zero GUID are rejected. Negative nonzero
    /// local IDs are preserved exactly. This value defines only catalog key
    /// validity; it does not define serialized wire or null semantics.
    pub fn new(guid: Guid, local_file_id: i64) -> Result<Self, InvalidReference> {
        if guid.is_zero() {
            return Err(InvalidReference::AllZeroGuid(guid));
        }

        if local_file_id == 0 {
            return Err(InvalidReference::ZeroLocalFileId(local_file_id));
        }

        Ok(Self {
            guid,
            local_file_id,
        })
    }

    /// Returns the asset GUID part of this catalog key.
    pub const fn guid(self) -> Guid {
        self.guid
    }

    /// Returns the signed local file ID part of this catalog key.
    pub const fn local_file_id(self) -> i64 {
        self.local_file_id
    }
}

/// A rejected asset-object reference component and its original typed value.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InvalidReference {
    /// The supplied GUID contained only zero bytes.
    AllZeroGuid(Guid),
    /// The supplied local file ID was zero.
    ZeroLocalFileId(i64),
}

impl InvalidReference {
    /// Returns which reference component failed validation.
    pub const fn component(self) -> InvalidReferenceComponent {
        match self {
            Self::AllZeroGuid(_) => InvalidReferenceComponent::Guid,
            Self::ZeroLocalFileId(_) => InvalidReferenceComponent::LocalFileId,
        }
    }

    /// Returns the rejected GUID when the GUID component failed validation.
    pub const fn rejected_guid(self) -> Option<Guid> {
        match self {
            Self::AllZeroGuid(guid) => Some(guid),
            Self::ZeroLocalFileId(_) => None,
        }
    }

    /// Returns the rejected local file ID when that component failed validation.
    pub const fn rejected_local_file_id(self) -> Option<i64> {
        match self {
            Self::AllZeroGuid(_) => None,
            Self::ZeroLocalFileId(local_file_id) => Some(local_file_id),
        }
    }
}

impl fmt::Display for InvalidReference {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::AllZeroGuid(guid) => {
                write!(f, "asset-object reference GUID is all zero: {guid}")
            }
            Self::ZeroLocalFileId(local_file_id) => {
                write!(f, "asset-object local file ID is zero: {local_file_id}")
            }
        }
    }
}

impl Error for InvalidReference {}

/// An explicit mesh or unlit-material resource selected by a catalog binding.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum AssetObjectTarget {
    /// A resource ID validated by `RenderAssets` for a mesh.
    Mesh(u32),
    /// A resource ID validated by `RenderAssets` for an unlit material.
    UnlitMaterial(u32),
}

impl AssetObjectTarget {
    /// Returns the target's typed class.
    pub const fn class(self) -> AssetObjectClass {
        match self {
            Self::Mesh(_) => AssetObjectClass::Mesh,
            Self::UnlitMaterial(_) => AssetObjectClass::UnlitMaterial,
        }
    }

    /// Returns the ID used to select the target in `RenderAssets`.
    pub const fn resource_id(self) -> u32 {
        match self {
            Self::Mesh(id) | Self::UnlitMaterial(id) => id,
        }
    }
}

/// One caller-supplied mapping from a catalog key to an explicit resource.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct AssetObjectBinding {
    reference: AssetObjectReference,
    target: AssetObjectTarget,
}

impl AssetObjectBinding {
    /// Creates a binding; catalog construction validates the target and key uniqueness.
    pub const fn new(reference: AssetObjectReference, target: AssetObjectTarget) -> Self {
        Self { reference, target }
    }

    /// Returns the exact reference key supplied for this binding.
    pub const fn reference(self) -> AssetObjectReference {
        self.reference
    }

    /// Returns the explicit resource target supplied for this binding.
    pub const fn target(self) -> AssetObjectTarget {
        self.target
    }
}
