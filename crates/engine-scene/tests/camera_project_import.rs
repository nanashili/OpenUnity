//! Public project-import acceptance for retained serialized Camera source data.

#[path = "support/camera_scene_fixture.rs"]
mod camera_scene_fixture;

use camera_scene_fixture::{camera_project_scene, replace_once};

use engine_scene::{
    CameraField, CameraSourceValue, MaterialField, PointerField, SerializedAssetPointer,
    import_project, import_project_with_render_references, import_project_with_render_sources,
};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

const SCENE_PATH: &str = "Assets/Scenes/Camera.unity";
const BINDINGS_PATH: &str = "Assets/Bindings.json";
const BINDINGS_JSON: &str = concat!(
    "{\"schema_version\":1,\"scripts\":[",
    "{\"guid\":\"0123456789abcdef0123456789abcdef\",",
    "\"assembly\":\"Assembly-CSharp\",\"type\":\"Fixture.Driver\",",
    "\"execution_order\":0,\"fields\":[]}]}",
);

#[test]
fn combined_import_retains_camera_sources_and_removes_serialized_components() {
    let fixture = ProjectFixture::new(&camera_project_scene());
    let imported = fixture.import_with_sources().unwrap();

    assert_eq!(imported.render.plan.roots, vec![10, 11]);
    assert_eq!(imported.render.plan.game_objects[0].components, vec![10]);
    assert_eq!(imported.render.plan.game_objects[1].components, vec![11]);
    assert_eq!(imported.render.render_references.mesh_filters.len(), 1);
    assert_eq!(
        imported.render.render_references.mesh_filters[0].mesh,
        PointerField::Pointer(SerializedAssetPointer {
            file_id: 700,
            guid: Some("Mesh-GUID".to_owned()),
            type_hint: Some(3),
        })
    );
    assert_eq!(imported.render.render_references.mesh_renderers.len(), 1);
    assert_eq!(
        imported.render.render_references.mesh_renderers[0].materials,
        MaterialField::Slots(Vec::new())
    );

    assert_camera_source_records(&imported.cameras.cameras);
}

#[test]
fn existing_importers_continue_to_reject_camera_documents() {
    let fixture = ProjectFixture::new(&camera_project_scene());

    let legacy_error = match import_project(
        &fixture.root,
        Path::new(SCENE_PATH),
        Path::new(BINDINGS_PATH),
    ) {
        Ok(_) => panic!("legacy import must reject serialized Camera documents"),
        Err(error) => error,
    };
    assert!(
        legacy_error
            .to_string()
            .contains("unsupported scene classID")
    );

    let render_error = match import_project_with_render_references(
        &fixture.root,
        Path::new(SCENE_PATH),
        Path::new(BINDINGS_PATH),
    ) {
        Ok(_) => panic!("render-reference import must reject serialized Camera documents"),
        Err(error) => error,
    };
    assert!(
        render_error
            .to_string()
            .contains("unsupported scene classID")
    );
}

#[test]
fn combined_import_rejects_invalid_camera_ownership_and_document_ids() {
    let valid_scene = camera_project_scene();
    let mismatched_owner = replace_once(
        &valid_scene,
        "Camera:\n  m_GameObject: {fileID: 100}",
        "Camera:\n  m_GameObject: {fileID: 200}",
    );
    let unlisted_camera = replace_once(&valid_scene, "  - component: {fileID: 31}\n", "");
    let duplicate_camera_id = replace_once(&valid_scene, "--- !u!20 &30\n", "--- !u!20 &31\n");
    let nonpositive_camera_id = replace_once(&valid_scene, "--- !u!20 &30\n", "--- !u!20 &0\n");

    let cases = [
        (
            "Camera owner mismatch",
            mismatched_owner,
            "Camera document 30 owner mismatch",
        ),
        (
            "Camera missing from its GameObject component list",
            unlisted_camera,
            "Camera document 31 is not listed by a GameObject",
        ),
        (
            "duplicate Camera document identity",
            duplicate_camera_id,
            "duplicate document fileID",
        ),
        (
            "nonpositive Camera document identity",
            nonpositive_camera_id,
            "nonpositive document fileID",
        ),
    ];

    assert_camera_import_failures(&cases);
}

#[test]
fn missing_mismatched_and_multiple_transforms_fail_both_validation_paths() {
    let valid_scene = camera_project_scene();
    let missing_transform = replace_once(&valid_scene, "  - component: {fileID: 10}\n", "");
    let mismatched_transform_owner = replace_once(
        &valid_scene,
        "  m_GameObject: {fileID: 100}\n",
        "  m_GameObject: {fileID: 200}\n",
    );

    let multiple_transforms = scene_with_multiple_transforms(&valid_scene);

    let cases = [
        (
            "Camera GameObject missing its listed Transform",
            missing_transform,
            "component list limit",
        ),
        (
            "Transform owner mismatch",
            mismatched_transform_owner,
            "component owner/list disagreement",
        ),
        (
            "multiple listed Transforms",
            multiple_transforms,
            "invalid graph size or mandatory Transform count",
        ),
    ];

    assert_camera_import_failures(&cases);

    let extraction_cases = [
        (
            cases[0].0,
            cases[0].1.as_str(),
            "GameObject document 100 is missing its Transform",
        ),
        (
            cases[1].0,
            cases[1].1.as_str(),
            "Transform document 10 owner mismatch",
        ),
        (
            cases[2].0,
            cases[2].1.as_str(),
            "GameObject document 100 has multiple Transform components",
        ),
    ];
    assert_camera_extraction_failures(&extraction_cases);
}

#[test]
fn scene_conversion_errors_precede_camera_extraction_errors() {
    let valid_scene = camera_project_scene();
    let mismatched_camera = replace_once(
        &valid_scene,
        "Camera:\n  m_GameObject: {fileID: 100}",
        "Camera:\n  m_GameObject: {fileID: 200}",
    );
    let unsupported_document = replace_once(
        &mismatched_camera,
        "--- !u!1660057539 &900\n",
        "--- !u!999 &901\nUnsupported:\n  value: 1\n--- !u!1660057539 &900\n",
    );
    let fixture = ProjectFixture::new(&unsupported_document);

    let error = fixture
        .import_with_sources()
        .err()
        .expect("unsupported scene class must fail conversion");

    assert!(error.to_string().contains("unsupported scene classID"));
    assert!(
        !error
            .to_string()
            .contains("Camera document 30 owner mismatch")
    );
}

#[test]
fn render_reference_errors_precede_camera_extraction_errors() {
    let valid_scene = camera_project_scene();
    let invalid_mesh = replace_once(
        &valid_scene,
        "m_Mesh: {fileID: 700, guid: Mesh-GUID, type: 3}",
        "m_Mesh: {fileID: \"700\"}",
    );
    let invalid_mesh_and_camera = replace_once(
        &invalid_mesh,
        "Camera:\n  m_GameObject: {fileID: 100}",
        "Camera:\n  m_GameObject: {fileID: 200}",
    );
    let fixture = ProjectFixture::new(&invalid_mesh_and_camera);

    let error = fixture
        .import_with_sources()
        .err()
        .expect("both source views must fail");
    let message = error.to_string();

    assert!(message.contains("MeshFilter.m_Mesh"));
    assert!(!message.contains("Camera document 30 owner mismatch"));
}

fn assert_camera_source_records(cameras: &[engine_scene::CameraSourceRecord]) {
    assert_eq!(cameras.len(), 2);
    assert_eq!(cameras[0].component_file_id, 31);
    assert_eq!(cameras[0].game_object_file_id, 200);
    assert_eq!(cameras[0].transform_file_id, 11);
    assert_eq!(
        cameras[0].properties.enabled,
        CameraField::Present {
            source_value: CameraSourceValue::Scalar("0".to_owned()),
            value: false,
        }
    );
    assert!(matches!(
        &cameras[0].transform_pose.local_position,
        CameraField::Present { value, .. } if value == &[4.0, 5.0, 6.0]
    ));
    assert_eq!(cameras[0].properties.aspect, None);
    let aspect = cameras[0]
        .uninterpreted_fields
        .iter()
        .find(|field| field.key == "aspect")
        .expect("unmapped aspect source key is retained");
    assert_eq!(
        aspect.source_value,
        CameraSourceValue::Scalar("1.777".to_owned())
    );

    assert_eq!(cameras[1].component_file_id, 30);
    assert_eq!(cameras[1].game_object_file_id, 100);
    assert_eq!(cameras[1].transform_file_id, 10);
    assert!(matches!(
        &cameras[1].properties.enabled,
        CameraField::Invalid { source_value: CameraSourceValue::Scalar(value), .. }
            if value == "unavailable"
    ));
    assert!(matches!(
        &cameras[1].properties.orthographic,
        CameraField::Invalid { source_value: CameraSourceValue::Scalar(value), .. }
            if value == "maybe"
    ));
    assert_eq!(cameras[1].properties.far_clip_plane, CameraField::Missing);
    assert!(matches!(
        &cameras[1].properties.field_of_view,
        CameraField::Invalid { source_value: CameraSourceValue::Scalar(value), .. }
            if value == "unknown"
    ));
}

fn assert_camera_import_failures(cases: &[(&str, String, &str)]) {
    for (case, scene, expected_error) in cases {
        let fixture = ProjectFixture::new(scene);
        let error = fixture
            .import_with_sources()
            .err()
            .unwrap_or_else(|| panic!("{case} must fail without returning an import"));

        assert!(
            error.to_string().contains(*expected_error),
            "{case}: unexpected error: {error}"
        );
    }
}

fn assert_camera_extraction_failures(cases: &[(&str, &str, &str)]) {
    for (case, scene, expected_error) in cases {
        let error = match engine_scene::extract_scene_cameras(scene.as_bytes()) {
            Ok(_) => panic!("{case} must fail Camera extraction"),
            Err(error) => error,
        };

        assert!(
            error.to_string().contains(*expected_error),
            "{case}: unexpected Camera extraction error: {error}"
        );
    }
}

fn scene_with_multiple_transforms(valid_scene: &str) -> String {
    let scene_with_multiple_memberships = replace_once(
        valid_scene,
        "  - component: {fileID: 10}\n",
        "  - component: {fileID: 10}\n  - component: {fileID: 12}\n",
    );

    replace_once(
        &scene_with_multiple_memberships,
        "--- !u!20 &31\n",
        &format!("{SECOND_TRANSFORM_DOCUMENT}--- !u!20 &31\n"),
    )
}

const SECOND_TRANSFORM_DOCUMENT: &str = concat!(
    "--- !u!4 &12\n",
    "Transform:\n",
    "  m_ObjectHideFlags: 0\n",
    "  m_CorrespondingSourceObject: {fileID: 0}\n",
    "  m_PrefabInstance: {fileID: 0}\n",
    "  m_PrefabAsset: {fileID: 0}\n",
    "  m_GameObject: {fileID: 100}\n",
    "  serializedVersion: 2\n",
    "  m_LocalRotation: {x: 0, y: 0, z: 0, w: 1}\n",
    "  m_LocalPosition: {x: 1, y: 2, z: 3}\n",
    "  m_LocalScale: {x: 1, y: 1, z: 1}\n",
    "  m_ConstrainProportionsScale: 0\n",
    "  m_Children: []\n",
    "  m_Father: {fileID: 0}\n",
    "  m_LocalEulerAnglesHint: {x: 0, y: 0, z: 0}\n",
);

struct ProjectFixture {
    root: PathBuf,
}

impl ProjectFixture {
    fn new(scene: &str) -> Self {
        static NEXT_FIXTURE_ID: AtomicU64 = AtomicU64::new(0);
        let path = std::env::temp_dir().join(format!(
            "openunity-camera-import-{}-{}",
            std::process::id(),
            NEXT_FIXTURE_ID.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).expect("create isolated camera-import project");
        let root = fs::canonicalize(path).expect("canonicalize isolated project root");
        let fixture = Self { root };

        fixture.write_text(SCENE_PATH, scene);
        fixture.write_text(
            "Assets/Scenes/Camera.unity.meta",
            "fileFormatVersion: 2\nguid: abcdef0123456789abcdef0123456789\n",
        );
        fixture.write_text("Assets/Fixtures/Driver.cs", "// Metadata target only.\n");
        fixture.write_text(
            "Assets/Fixtures/Driver.cs.meta",
            concat!(
                "fileFormatVersion: 2\n",
                "guid: 0123456789abcdef0123456789abcdef\n"
            ),
        );
        fixture.write_text(BINDINGS_PATH, BINDINGS_JSON);

        fixture
    }

    fn write_text(&self, relative_path: &str, contents: &str) {
        let path = self.root.join(relative_path);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("create project fixture directory");
        }
        fs::write(path, contents).expect("write project fixture file");
    }

    fn import_with_sources(
        &self,
    ) -> std::result::Result<engine_scene::ProjectRenderSourceImport, engine_scene::SceneError>
    {
        import_project_with_render_sources(
            &self.root,
            Path::new(SCENE_PATH),
            Path::new(BINDINGS_PATH),
        )
    }
}

impl Drop for ProjectFixture {
    fn drop(&mut self) {
        if let Err(error) = fs::remove_dir_all(&self.root) {
            if std::thread::panicking() {
                eprintln!("camera-import fixture cleanup failed during unwinding: {error}");
            } else {
                panic!("camera-import fixture cleanup failed: {error}");
            }
        }
    }
}
