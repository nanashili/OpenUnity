use super::{RenderManifest, RenderManifestError};
use engine_assets::MetadataIndex;
use engine_scene::{
    CameraField, CameraSourceProperties, CameraSourceRecord, CameraSourceValue, GameObjectPlan,
    ProjectRenderImport, ProjectRenderSourceImport, SceneCameraSourceRecords, ScenePlan,
    SceneRenderReferences, TransformPlan, TransformSourcePose,
};

const VALID_MANIFEST: &str = r#"
{
  "schema_version": 1,
  "meshes": [
    {
      "id": 17,
      "positions": [[0, 0, 0], [1, 0, 0], [0, 1, 0]],
      "indices": [0, 1, 2]
    }
  ],
  "materials": [{"id": 23, "rgba8": [255, 255, 255, 255]}],
  "camera": {
    "owner": 1,
    "half_height": 1,
    "near": 0.1,
    "far": 100,
    "clear_rgba8": [0, 0, 0, 255]
  },
  "draws": [{"owner": 2, "mesh": 17, "material": 23, "enabled": true}]
}
"#;

#[test]
fn parses_borrowed_assets_and_resolves_draw_bindings() {
    let manifest = RenderManifest::parse_json(VALID_MANIFEST.as_bytes()).unwrap();

    assert_eq!(manifest.assets().meshes().len(), 1);
    assert_eq!(manifest.assets().meshes()[0].id(), 17);
    assert_eq!(manifest.assets().materials().len(), 1);
    assert_eq!(manifest.assets().materials()[0].id(), 23);
    assert_eq!(manifest.camera().owner(), 1);
    assert_eq!(manifest.draws().len(), 1);
    assert_eq!(manifest.draws()[0].owner(), 2);
    assert_eq!(manifest.draws()[0].mesh_index(), 0);
    assert_eq!(manifest.draws()[0].material_index(), 0);
}

#[test]
fn rejects_duplicate_root_keys_before_asset_parsing() {
    let duplicate = VALID_MANIFEST.replacen(
        "\"schema_version\": 1,",
        "\"schema_version\": 1, \"schema_version\": 1,",
        1,
    );

    assert_eq!(
        RenderManifest::parse_json(duplicate.as_bytes()).unwrap_err(),
        RenderManifestError::DuplicateKey
    );
}

#[test]
fn admits_the_exact_draw_limit() {
    let manifest = manifest_with_draws(valid_draws(4096));

    let parsed = RenderManifest::parse_json(manifest.as_bytes()).unwrap();

    assert_eq!(parsed.draws().len(), 4096);
}

#[test]
fn rejects_an_extra_draw_before_decoding_its_duplicate_key() {
    let mut draws = valid_draws(4096);
    draws.push(r#"{"owner": 4098, "owner": 4099}"#.into());
    let manifest = manifest_with_draws(draws);

    assert_eq!(
        RenderManifest::parse_json(manifest.as_bytes()).unwrap_err(),
        RenderManifestError::Limit("draws")
    );
}

fn valid_draws(count: usize) -> Vec<String> {
    (0..count)
        .map(|index| {
            let owner = index + 2;
            format!(r#"{{"owner": {owner}, "mesh": 17, "material": 23, "enabled": true}}"#)
        })
        .collect()
}

fn manifest_with_draws(draws: Vec<String>) -> String {
    let draw_list = draws.join(", ");
    VALID_MANIFEST.replace(
        r#""draws": [{"owner": 2, "mesh": 17, "material": 23, "enabled": true}]"#,
        &format!(r#""draws": [{draw_list}]"#),
    )
}

#[test]
fn reports_unknown_camera_fields_even_with_escaped_quotes_and_braces() {
    for key in [r#"extra["#, r#"extra\"["#, r#"extra\\["#] {
        let manifest =
            VALID_MANIFEST.replace(r#""owner": 1,"#, &format!(r#""{key}": 0, "owner": 1,"#));
        assert_eq!(
            RenderManifest::parse_json(manifest.as_bytes()).unwrap_err(),
            RenderManifestError::UnknownField("camera"),
            "key: {key}",
        );
    }
}

#[test]
fn version2_uses_manifest_projection_for_a_disabled_source_camera() {
    let (plan, manifest) = parse_manifest_with_cameras(10, vec![source_camera(10)]).unwrap();
    manifest
        .validate_bindings(&plan)
        .expect("the camera owner's GameObject is part of the imported plan");

    let camera = manifest.camera();
    assert_eq!(camera.owner(), 10);
    assert_eq!(camera.half_height(), 1.25);
    assert_eq!(camera.near(), 0.5);
    assert_eq!(camera.far(), 250.0);
    assert_eq!(camera.clear_rgba8(), [12, 24, 36, 255]);
}

#[test]
fn version2_rejects_unrelated_and_component_camera_owners() {
    for camera_owner in [30, 99] {
        let error = parse_manifest_with_cameras(camera_owner, vec![source_camera(10)])
            .expect_err("the owner must be a source Camera's GameObject ID");

        assert_eq!(
            error,
            RenderManifestError::InvalidField("camera.owner source"),
            "camera owner: {camera_owner}",
        );
    }
}

#[test]
fn version2_keeps_game_object_validation_when_no_source_cameras_exist() {
    let (plan, manifest) = parse_manifest_with_cameras(10, Vec::new()).unwrap();

    manifest
        .validate_bindings(&plan)
        .expect("the pre-existing GameObject owner route remains valid without source Cameras");
    assert_eq!(manifest.camera().owner(), 10);
}

fn parse_manifest_with_cameras(
    camera_owner: i64,
    cameras: Vec<CameraSourceRecord>,
) -> Result<(ScenePlan, RenderManifest), RenderManifestError> {
    let plan = scene_plan();
    let imported = ProjectRenderSourceImport {
        render: ProjectRenderImport {
            plan: plan.clone(),
            render_references: SceneRenderReferences {
                mesh_filters: Vec::new(),
                mesh_renderers: Vec::new(),
            },
            metadata: MetadataIndex::new(),
        },
        cameras: SceneCameraSourceRecords { cameras },
    };

    let manifest_bytes = version2_manifest(camera_owner);
    let (_, manifest) = RenderManifest::parse_version2(&manifest_bytes, imported)?;
    Ok((plan, manifest))
}

fn version2_manifest(camera_owner: i64) -> Vec<u8> {
    format!(
        r#"{{
            "schema_version": 2,
            "meshes": [],
            "materials": [],
            "camera": {{
                "owner": {camera_owner},
                "half_height": 1.25,
                "near": 0.5,
                "far": 250.0,
                "clear_rgba8": [12, 24, 36, 255]
            }},
            "draws": [],
            "asset_bindings": []
        }}"#
    )
    .into_bytes()
}

fn scene_plan() -> ScenePlan {
    ScenePlan {
        schema_version: 1,
        scene_name: "camera-binding-test".to_owned(),
        roots: vec![20, 40],
        game_objects: vec![
            GameObjectPlan {
                file_id: 10,
                name: "camera-owner".to_owned(),
                active: true,
                components: vec![20],
            },
            GameObjectPlan {
                file_id: 99,
                name: "unrelated-camera-candidate".to_owned(),
                active: true,
                components: vec![40],
            },
        ],
        transforms: vec![
            TransformPlan {
                file_id: 20,
                owner: 10,
                parent: 0,
                children: Vec::new(),
                local_position: [0.0; 3],
                local_rotation: [0.0, 0.0, 0.0, 1.0],
                local_scale: [1.0; 3],
            },
            TransformPlan {
                file_id: 40,
                owner: 99,
                parent: 0,
                children: Vec::new(),
                local_position: [0.0; 3],
                local_rotation: [0.0, 0.0, 0.0, 1.0],
                local_scale: [1.0; 3],
            },
        ],
        scripts: Vec::new(),
    }
}

fn source_camera(game_object_file_id: i64) -> CameraSourceRecord {
    CameraSourceRecord {
        component_file_id: 30,
        game_object_file_id,
        transform_file_id: 20,
        properties: CameraSourceProperties {
            enabled: CameraField::Present {
                source_value: CameraSourceValue::Scalar("0".to_owned()),
                value: false,
            },
            orthographic: CameraField::Invalid {
                source_value: CameraSourceValue::Scalar("unknown".to_owned()),
                diagnostic: "orthographic: invalid boolean".to_owned(),
            },
            aspect: None,
            orthographic_size: CameraField::Present {
                source_value: CameraSourceValue::Scalar("99".to_owned()),
                value: 99.0,
            },
            field_of_view: CameraField::Missing,
            near_clip_plane: CameraField::Invalid {
                source_value: CameraSourceValue::Scalar("invalid".to_owned()),
                diagnostic: "near clip plane: invalid number".to_owned(),
            },
            far_clip_plane: CameraField::Present {
                source_value: CameraSourceValue::Scalar("0.01".to_owned()),
                value: 0.01,
            },
            depth: CameraField::Missing,
        },
        transform_pose: TransformSourcePose {
            local_position: CameraField::Missing,
            local_rotation: CameraField::Missing,
            local_scale: CameraField::Missing,
        },
        uninterpreted_fields: Vec::new(),
    }
}
