//! Compatibility tests for the supported saved-scene directive headers.

#[path = "support/camera_scene_fixture.rs"]
mod camera_scene_fixture;

use camera_scene_fixture::{LEGACY_HEADER, camera_project_scene, replace_once, scene_with_header};
use engine_scene::{MAX_DEPTH, MAX_INPUT_BYTES, MAX_OBJECTS, extract_scene_render_references};

const TWO_LINE_HEADER: &str = concat!("%YAML 1.1\n", "%TAG !u! tag:unity3d.com,2011:\n");

#[test]
fn accepts_two_line_and_legacy_headers_with_equal_scene_records() {
    let two_line_scene = scene_with_header(TWO_LINE_HEADER);
    let legacy_scene = camera_project_scene();

    let two_line_records = extract_scene_render_references(two_line_scene.as_bytes()).unwrap();
    let legacy_records = extract_scene_render_references(legacy_scene.as_bytes()).unwrap();

    assert_eq!(two_line_records, legacy_records);
    assert_eq!(two_line_records.mesh_filters.len(), 1);
    assert_eq!(two_line_records.mesh_renderers.len(), 1);
}

#[test]
fn rejects_malformed_directive_headers() {
    let unsupported_headers = [
        (
            "wrong YAML version",
            "%YAML 1.2\n%TAG !u! tag:unity3d.com,2011:\n",
        ),
        (
            "wrong tag handle",
            "%YAML 1.1\n%TAG !x! tag:unity3d.com,2011:\n",
        ),
        (
            "reversed directives",
            "%TAG !u! tag:unity3d.com,2011:\n%YAML 1.1\n",
        ),
        ("missing YAML directive", "%TAG !u! tag:unity3d.com,2011:\n"),
        ("missing tag directive", "%YAML 1.1\n"),
        (
            "extra directive after the pair",
            "%YAML 1.1\n%TAG !u! tag:unity3d.com,2011:\n%UNITY 7.0\n",
        ),
        (
            "misplaced Unity directive",
            "%YAML 1.1\n%UNITY 7.0\n%TAG !u! tag:unity3d.com,2011:\n",
        ),
    ];

    for (case, header) in unsupported_headers {
        let scene = scene_with_header(header);
        let error = extract_scene_render_references(scene.as_bytes())
            .expect_err("unsupported saved-scene directive headers must fail");

        assert!(
            error
                .to_string()
                .contains("unsupported saved scene directives"),
            "{case}: unexpected error: {error}"
        );
    }
}

#[test]
fn malformed_scene_body_fails_identically_under_both_supported_headers() {
    let malformed_scene = replace_once(
        &camera_project_scene(),
        "  m_ObjectHideFlags: 0\n",
        "   m_ObjectHideFlags: 0\n",
    );
    let body = malformed_scene
        .strip_prefix(LEGACY_HEADER)
        .expect("legacy fixture begins with its documented directive prefix");

    assert_identical_body_error(body, "unsupported indentation");
}

#[test]
fn scene_size_depth_and_document_limits_are_header_independent() {
    let oversized_body = "x".repeat(MAX_INPUT_BYTES + 1);
    assert_identical_body_error(&oversized_body, "scene byte limit");

    let overdeep_body = format!(
        "--- !u!1 &1\nGameObject:\n{}m_Name: too-deep\n",
        " ".repeat((MAX_DEPTH + 1) * 2),
    );
    assert_identical_body_error(&overdeep_body, "unsupported indentation");

    let too_many_documents = (1..=MAX_OBJECTS + 1)
        .map(|file_id| format!("--- !u!999 &{file_id}\nUnknown:\n  value: 1\n"))
        .collect::<String>();
    assert_identical_body_error(&too_many_documents, "document count limit");
}

fn assert_identical_body_error(body: &str, expected_error: &str) {
    let legacy_scene = format!("{LEGACY_HEADER}{body}");
    let two_line_scene = format!("{TWO_LINE_HEADER}{body}");
    let legacy_error = extract_scene_render_references(legacy_scene.as_bytes())
        .expect_err("invalid scene body must fail under the legacy header");
    let two_line_error = extract_scene_render_references(two_line_scene.as_bytes())
        .expect_err("invalid scene body must fail under the two-line header");

    assert_eq!(legacy_error.to_string(), two_line_error.to_string());
    assert!(
        legacy_error.to_string().contains(expected_error),
        "unexpected error: {legacy_error}"
    );
}
