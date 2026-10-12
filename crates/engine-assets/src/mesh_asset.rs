use std::fmt;

pub(super) const MAX_MESH_POSITIONS: usize = 65_536;
pub(super) const MAX_MESH_INDICES: usize = 196_608;

#[derive(Clone, Debug, Eq, PartialEq)]
/// Errors produced while validating or parsing render assets.
pub enum AssetError {
    /// The serialized manifest or value has invalid syntax or shape.
    Parse(String),
    /// An object contains the same JSON key more than once.
    DuplicateKey,
    /// Two assets of the same kind use the same identifier.
    DuplicateId(&'static str),
    /// A serialized or typed asset exceeds an approved bound.
    Limit(&'static str),
    /// A numeric value is outside the supported finite or encoding range.
    Numeric(&'static str),
    /// An index or other asset reference points outside its target.
    Reference(&'static str),
}

impl fmt::Display for AssetError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Parse(detail) => write!(formatter, "asset parse error: {detail}"),
            Self::DuplicateKey => formatter.write_str("duplicate JSON key"),
            Self::DuplicateId(kind) => write!(formatter, "duplicate {kind} ID"),
            Self::Limit(kind) => write!(formatter, "asset {kind} limit"),
            Self::Numeric(kind) => write!(formatter, "invalid {kind} number"),
            Self::Reference(kind) => write!(formatter, "invalid {kind} reference"),
        }
    }
}

impl std::error::Error for AssetError {}

/// Validated indexed triangle geometry owned by the asset domain.
#[derive(Clone, Debug, PartialEq)]
pub struct MeshAsset {
    id: u32,
    positions: Vec<[f32; 3]>,
    indices: Vec<u32>,
}

impl MeshAsset {
    /// Creates finite indexed triangle geometry with a nonzero u16-range ID.
    pub fn try_new(
        id: u32,
        positions: Vec<[f32; 3]>,
        indices: Vec<u32>,
    ) -> Result<Self, AssetError> {
        if id == 0 || id > u16::MAX as u32 {
            return Err(AssetError::Limit("mesh ID"));
        }
        if !(3..=MAX_MESH_POSITIONS).contains(&positions.len()) {
            return Err(AssetError::Limit("mesh positions"));
        }
        if !(3..=MAX_MESH_INDICES).contains(&indices.len()) || indices.len() % 3 != 0 {
            return Err(AssetError::Limit("mesh indices"));
        }
        if !positions.iter().flatten().all(|value| value.is_finite()) {
            return Err(AssetError::Numeric("mesh position"));
        }
        if indices
            .iter()
            .any(|index| *index as usize >= positions.len())
        {
            return Err(AssetError::Reference("mesh index"));
        }
        Ok(Self {
            id,
            positions,
            indices,
        })
    }

    /// Returns the stable asset identifier.
    pub fn id(&self) -> u32 {
        self.id
    }

    /// Returns vertex positions as xyz triples.
    pub fn positions(&self) -> &[[f32; 3]] {
        &self.positions
    }

    /// Returns triangle indices in groups of three.
    pub fn indices(&self) -> &[u32] {
        &self.indices
    }
}
