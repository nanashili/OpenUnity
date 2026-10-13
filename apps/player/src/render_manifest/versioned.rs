use super::{
    CameraBinding, DrawBinding, RenderManifest, RenderManifestError, RenderResources, scene_bound,
};
use crate::scene_reference_binding::{AssetBindingRow, BindingKind, resolve_scene_bindings};
use engine_scene::ProjectRenderImport;

impl RenderManifest {
    pub fn parse_version2(
        bytes: &[u8],
        imported: ProjectRenderImport,
    ) -> Result<(engine_scene::ScenePlan, Self), RenderManifestError> {
        let ProjectRenderImport {
            plan,
            render_references,
            metadata,
        } = imported;
        let wire = scene_bound::parse(bytes).map_err(|error| match error {
            scene_bound::V2WireError::InvalidJson => {
                RenderManifestError::Parse("invalid JSON".into())
            }
            scene_bound::V2WireError::DuplicateKey => RenderManifestError::DuplicateKey,
            scene_bound::V2WireError::InvalidSchema => {
                RenderManifestError::InvalidField("render manifest structure")
            }
            scene_bound::V2WireError::UnknownField => {
                RenderManifestError::UnknownField("render manifest")
            }
            scene_bound::V2WireError::InvalidField(field) => {
                RenderManifestError::InvalidField(field)
            }
            scene_bound::V2WireError::Asset(error) => RenderManifestError::Asset(error.to_string()),
            scene_bound::V2WireError::Limit(limit) => RenderManifestError::Limit(limit),
        })?;
        let camera = CameraBinding {
            owner: wire.camera.owner,
            half_height: wire.camera.half_height,
            near: wire.camera.near,
            far: wire.camera.far,
            clear_rgba8: wire.camera.clear_rgba8,
        };
        let draw_owners: Vec<_> = wire.draws.iter().map(|draw| draw.owner).collect();
        let rows = wire
            .asset_bindings
            .iter()
            .map(|binding| {
                let kind = match binding.kind.as_str() {
                    "mesh" => BindingKind::Mesh,
                    "unlit_material" => BindingKind::UnlitMaterial,
                    _ => return Err(RenderManifestError::InvalidField("asset_bindings.kind")),
                };

                Ok(AssetBindingRow {
                    guid: binding.guid.clone(),
                    local_file_id: binding.local_file_id,
                    kind,
                    resource_id: binding.resource_id,
                })
            })
            .collect::<Result<Vec<_>, _>>()?;
        let resolved = resolve_scene_bindings(
            metadata,
            wire.assets,
            &render_references,
            &draw_owners,
            &rows,
        )
        .map_err(RenderManifestError::Reference)?;
        let draws = wire
            .draws
            .iter()
            .zip(resolved.draws())
            .map(|(wire, resolved)| DrawBinding {
                owner: resolved.owner(),
                mesh_index: resolved.mesh_index(),
                material_index: resolved.material_index(),
                enabled: wire.enabled,
            })
            .collect();

        Ok((
            plan,
            Self {
                resources: RenderResources::Catalog(resolved),
                camera,
                draws,
            },
        ))
    }
}
