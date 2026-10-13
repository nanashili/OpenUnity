use super::{
    CameraBinding, DrawBinding, RenderManifest, RenderManifestError, RenderResources, scene_bound,
};
use crate::scene_reference_binding::{
    AssetBindingRow, BindingKind, ResolvedSceneBindings, resolve_scene_bindings,
};
use engine_scene::{ProjectRenderImport, ProjectRenderSourceImport, SceneCameraSourceRecords};

struct ResolvedVersion2Scene {
    plan: engine_scene::ScenePlan,
    resources: ResolvedSceneBindings,
    draws: Vec<DrawBinding>,
}

impl RenderManifest {
    pub fn parse_version2(
        bytes: &[u8],
        imported: ProjectRenderSourceImport,
    ) -> Result<(engine_scene::ScenePlan, Self), RenderManifestError> {
        let ProjectRenderSourceImport { render, cameras } = imported;
        let wire = parse_version2_wire(bytes)?;

        validate_camera_source_owner(wire.camera.owner, &cameras)?;

        let camera = CameraBinding {
            owner: wire.camera.owner,
            half_height: wire.camera.half_height,
            near: wire.camera.near,
            far: wire.camera.far,
            clear_rgba8: wire.camera.clear_rgba8,
        };
        let scene = resolve_version2_scene(render, wire)?;

        Ok((
            scene.plan,
            Self {
                resources: RenderResources::Catalog(scene.resources),
                camera,
                draws: scene.draws,
            },
        ))
    }
}

fn parse_version2_wire(bytes: &[u8]) -> Result<scene_bound::V2Manifest<'_>, RenderManifestError> {
    scene_bound::parse(bytes).map_err(|error| match error {
        scene_bound::V2WireError::InvalidJson => RenderManifestError::Parse("invalid JSON".into()),
        scene_bound::V2WireError::DuplicateKey => RenderManifestError::DuplicateKey,
        scene_bound::V2WireError::InvalidSchema => {
            RenderManifestError::InvalidField("render manifest structure")
        }
        scene_bound::V2WireError::UnknownField => {
            RenderManifestError::UnknownField("render manifest")
        }
        scene_bound::V2WireError::InvalidField(field) => RenderManifestError::InvalidField(field),
        scene_bound::V2WireError::Asset(error) => RenderManifestError::Asset(error.to_string()),
        scene_bound::V2WireError::Limit(limit) => RenderManifestError::Limit(limit),
    })
}

fn validate_camera_source_owner(
    owner: i64,
    cameras: &SceneCameraSourceRecords,
) -> Result<(), RenderManifestError> {
    if cameras.cameras.is_empty() {
        return Ok(());
    }

    let owner_matches_source_camera = cameras
        .cameras
        .iter()
        .any(|camera| camera.game_object_file_id == owner);
    if owner_matches_source_camera {
        Ok(())
    } else {
        Err(RenderManifestError::InvalidField("camera.owner source"))
    }
}

fn resolve_version2_scene(
    imported: ProjectRenderImport,
    wire: scene_bound::V2Manifest<'_>,
) -> Result<ResolvedVersion2Scene, RenderManifestError> {
    let ProjectRenderImport {
        plan,
        render_references,
        metadata,
    } = imported;
    let scene_bound::V2Manifest {
        assets,
        draws: wire_draws,
        asset_bindings,
        ..
    } = wire;

    let draw_owners = wire_draws.iter().map(|draw| draw.owner).collect::<Vec<_>>();
    let rows = asset_binding_rows(&asset_bindings)?;
    let resources =
        resolve_scene_bindings(metadata, assets, &render_references, &draw_owners, &rows)
            .map_err(RenderManifestError::Reference)?;
    let draws = resolve_draw_bindings(&wire_draws, &resources);

    Ok(ResolvedVersion2Scene {
        plan,
        resources,
        draws,
    })
}

fn asset_binding_rows(
    bindings: &[scene_bound::V2AssetBinding],
) -> Result<Vec<AssetBindingRow>, RenderManifestError> {
    bindings
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
        .collect()
}

fn resolve_draw_bindings(
    wire_draws: &[scene_bound::V2Draw],
    resources: &ResolvedSceneBindings,
) -> Vec<DrawBinding> {
    wire_draws
        .iter()
        .zip(resources.draws())
        .map(|(wire, resolved)| DrawBinding {
            owner: resolved.owner(),
            mesh_index: resolved.mesh_index(),
            material_index: resolved.material_index(),
            enabled: wire.enabled,
        })
        .collect()
}
