//! Bounded render-manifest parsing for the player route.
//!
//! This module owns camera and draw bindings. Mesh and material grammar and
//! geometry bounds remain in `engine-assets`; `RenderAssets::parse_parts`
//! receives borrowed array fragments without a player-side value tree, clone,
//! or synthetic JSON document.

use crate::scene_reference_binding::{PointerResolutionError, ResolvedSceneBindings};
use engine_assets::RenderAssets;

const MAX_BYTES: usize = 16 * 1024 * 1024;
const MAX_DEPTH: usize = 64;
const MAX_DRAWS: usize = 4096;

/// Validated camera values retained by the player route.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CameraBinding {
    owner: i64,
    half_height: f32,
    near: f32,
    far: f32,
    clear_rgba8: [u8; 4],
}

/// One ordered binding with resolved asset indices.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DrawBinding {
    owner: i64,
    mesh_index: u32,
    material_index: u32,
    enabled: bool,
}

/// Complete validated render manifest, including immutable asset values.
#[derive(Debug)]
pub struct RenderManifest {
    resources: RenderResources,
    camera: CameraBinding,
    draws: Vec<DrawBinding>,
}

#[derive(Debug)]
enum RenderResources {
    Direct(RenderAssets),
    Catalog(ResolvedSceneBindings),
}

/// Manifest input or validation failure.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RenderManifestError {
    /// A player-owned byte, nesting, collection, or value-tree bound was exceeded.
    Limit(&'static str),
    /// The byte stream is malformed JSON or a JSON value cannot be decoded.
    Parse(String),
    /// An object repeats a key, including a key whose escaped spelling matches.
    DuplicateKey,
    /// An object contains a member outside the reviewed schema.
    UnknownField(&'static str),
    /// A required value, number, identity, range, or reference is invalid.
    InvalidField(&'static str),
    /// The independently owned mesh/material parser rejected its asset subset.
    Asset(String),
    /// Scene-extracted references could not be paired or resolved by the catalog.
    Reference(PointerResolutionError),
}

pub(super) fn version2_preflight_error(error: RenderManifestError) -> String {
    match error {
        RenderManifestError::Reference(error) => error.to_string(),
        RenderManifestError::Limit(limit) => {
            format!("parse render manifest: limit exceeded for {limit}")
        }
        RenderManifestError::Parse(detail) => {
            format!("parse render manifest: {detail}")
        }
        RenderManifestError::DuplicateKey => "parse render manifest: duplicate object key".into(),
        RenderManifestError::UnknownField(scope) => {
            format!("parse render manifest: unknown field in {scope}")
        }
        RenderManifestError::InvalidField(field) => {
            format!("parse render manifest: invalid field {field}")
        }
        RenderManifestError::Asset(detail) => {
            format!("parse render manifest assets: {detail}")
        }
    }
}

impl RenderManifest {
    /// Parse and validate the bounded version-one render manifest.
    ///
    /// The original manifest is limited to 16 MiB and nesting depth 64.
    /// `engine-assets` validates borrowed mesh/material fragments and their
    /// geometry bounds. Camera and draw value trees each have separate limits
    /// of 2,000,000 nodes and 20,000 object members. Duplicate or unknown keys,
    /// unresolved asset references, invalid scene IDs, and invalid ranges fail.
    pub fn parse_json(bytes: &[u8]) -> Result<Self, RenderManifestError> {
        let wire = parse_manifest(bytes)?;
        if wire.schema_version != 1 {
            return Err(RenderManifestError::InvalidField("schema_version"));
        }
        let camera = parse_camera(wire.camera.get().as_bytes())?;
        let draws = parse_draws(wire.draws.get().as_bytes())?;
        let assets = RenderAssets::parse_parts(
            wire.schema_version,
            wire.meshes.get().as_bytes(),
            wire.materials.get().as_bytes(),
        )
        .map_err(|error| RenderManifestError::Asset(error.to_string()))?;
        let draws = resolve_draws(draws, &assets)?;

        Ok(Self {
            resources: RenderResources::Direct(assets),
            camera,
            draws,
        })
    }

    /// Return the independently validated mesh and material values.
    pub fn assets(&self) -> &RenderAssets {
        match &self.resources {
            RenderResources::Direct(assets) => assets,
            RenderResources::Catalog(bindings) => bindings.catalog().render_assets(),
        }
    }

    /// Return the validated camera owner and orthographic parameters.
    pub const fn camera(&self) -> CameraBinding {
        self.camera
    }

    /// Return draw bindings in authored order.
    pub fn draws(&self) -> &[DrawBinding] {
        &self.draws
    }

    /// Return camera then draw owners once, preserving first authored order.
    pub fn snapshot_owner_order(&self) -> Vec<i64> {
        let mut owners = Vec::with_capacity(self.draws.len() + 1);
        owners.push(self.camera.owner);
        for draw in &self.draws {
            if !owners.contains(&draw.owner) {
                owners.push(draw.owner);
            }
        }
        owners
    }

    /// Validate binding ownership against the imported scene plan.
    pub fn validate_bindings(
        &self,
        plan: &engine_scene::ScenePlan,
    ) -> Result<(), engine_scene::SceneError> {
        let draw_owners: Vec<_> = self.draws.iter().map(|draw| draw.owner).collect();
        engine_scene::validate_render_binding_owners(plan, self.camera.owner, &draw_owners)
    }
}

impl CameraBinding {
    /// Return the positive scene GameObject ID bound as the camera.
    pub const fn owner(self) -> i64 {
        self.owner
    }

    /// Return the positive orthographic half-height.
    pub const fn half_height(self) -> f32 {
        self.half_height
    }

    /// Return the positive near clip distance.
    pub const fn near(self) -> f32 {
        self.near
    }

    /// Return the far clip distance, greater than [`Self::near`].
    pub const fn far(self) -> f32 {
        self.far
    }

    /// Return the opaque RGBA8 camera clear color.
    pub const fn clear_rgba8(self) -> [u8; 4] {
        self.clear_rgba8
    }
}

impl DrawBinding {
    /// Return the unique positive scene GameObject ID owning the draw.
    pub const fn owner(self) -> i64 {
        self.owner
    }

    /// Return the resolved zero-based mesh index.
    pub const fn mesh_index(self) -> u32 {
        self.mesh_index
    }

    /// Return the resolved zero-based material index.
    pub const fn material_index(self) -> u32 {
        self.material_index
    }

    /// Return whether the authored draw is enabled.
    pub const fn enabled(self) -> bool {
        self.enabled
    }
}

mod parsing;
use parsing::{parse_camera, parse_draws, parse_manifest, resolve_draws};

mod scene_bound;
pub(crate) mod version_selection;
mod versioned;

#[cfg(test)]
#[path = "render_manifest_tests.rs"]
mod tests;
