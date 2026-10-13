use engine_assets::AssetObjectCatalog;
use std::fmt::{self, Display, Formatter};

/// One caller-authored mapping from serialized identity to an existing resource.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AssetBindingRow {
    pub guid: String,
    pub local_file_id: i64,
    pub kind: BindingKind,
    pub resource_id: u32,
}

/// The resource class selected by a caller-authored binding row.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BindingKind {
    Mesh,
    UnlitMaterial,
}

/// One draw with indices into the catalog-owned resource snapshot.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ResolvedDraw {
    pub(super) owner: i64,
    pub(super) mesh_index: u32,
    pub(super) material_index: u32,
}

impl ResolvedDraw {
    pub const fn owner(self) -> i64 {
        self.owner
    }
    pub const fn mesh_index(self) -> u32 {
        self.mesh_index
    }
    pub const fn material_index(self) -> u32 {
        self.material_index
    }
}

/// The immutable catalog and all draw projections derived from it.
#[derive(Debug)]
pub struct ResolvedSceneBindings {
    pub(super) catalog: AssetObjectCatalog,
    pub(super) draws: Vec<ResolvedDraw>,
}

impl ResolvedSceneBindings {
    pub fn catalog(&self) -> &AssetObjectCatalog {
        &self.catalog
    }
    pub fn draws(&self) -> &[ResolvedDraw] {
        &self.draws
    }
}

/// A stable, phase-labelled failure from scene pointer resolution.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PointerResolutionError(String);

impl Display for PointerResolutionError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl std::error::Error for PointerResolutionError {}

impl PointerResolutionError {
    pub(super) fn new(detail: String) -> Self {
        Self(detail)
    }
}
