//! Public-boundary fixtures for scene render-reference extraction.

use engine_scene::{
    MaterialField, PointerField, SerializedAssetPointer, extract_scene_render_references,
};

#[test]
fn extracts_mesh_and_ordered_material_references() {
    let scene = scene(
        Some("{fileID: 700, guid: \"Mesh-GUID\", type: 3}"),
        Some("\n    - {fileID: 701, guid: Material-GUID, type: 2}\n    - {fileID: 0}"),
    );

    let references = extract_scene_render_references(scene.as_bytes()).unwrap();

    assert_eq!(references.mesh_filters.len(), 1);
    assert_eq!(references.mesh_renderers.len(), 1);
    assert_eq!(references.mesh_filters[0].component_file_id, 200);
    assert_eq!(references.mesh_filters[0].game_object_file_id, 100);
    assert_eq!(
        references.mesh_filters[0].mesh,
        PointerField::Pointer(SerializedAssetPointer {
            file_id: 700,
            guid: Some("Mesh-GUID".to_owned()),
            type_hint: Some(3),
        })
    );
    assert_eq!(
        references.mesh_renderers[0].materials,
        MaterialField::Slots(vec![
            SerializedAssetPointer {
                file_id: 701,
                guid: Some("Material-GUID".to_owned()),
                type_hint: Some(2),
            },
            SerializedAssetPointer {
                file_id: 0,
                guid: None,
                type_hint: None,
            },
        ])
    );
}

#[test]
fn distinguishes_missing_null_zero_and_empty_slots() {
    let missing = extract_scene_render_references(scene(None, None).as_bytes()).unwrap();
    assert_eq!(missing.mesh_filters[0].mesh, PointerField::Missing);
    assert_eq!(missing.mesh_renderers[0].materials, MaterialField::Missing);

    let explicit_null =
        extract_scene_render_references(scene(Some("null"), Some(" null")).as_bytes()).unwrap();
    assert_eq!(
        explicit_null.mesh_filters[0].mesh,
        PointerField::ExplicitNull
    );
    assert_eq!(
        explicit_null.mesh_renderers[0].materials,
        MaterialField::ExplicitNull
    );

    let explicit_tilde =
        extract_scene_render_references(scene(Some("~"), Some(" ~")).as_bytes()).unwrap();
    assert_eq!(
        explicit_tilde.mesh_filters[0].mesh,
        PointerField::ExplicitNull
    );
    assert_eq!(
        explicit_tilde.mesh_renderers[0].materials,
        MaterialField::ExplicitNull
    );

    let empty_slots = extract_scene_render_references(scene(None, Some(" []")).as_bytes()).unwrap();
    assert_eq!(
        empty_slots.mesh_renderers[0].materials,
        MaterialField::Slots(vec![])
    );

    let zero_is_pointer =
        extract_scene_render_references(scene(Some("{fileID: 0}"), None).as_bytes()).unwrap();
    assert_eq!(
        zero_is_pointer.mesh_filters[0].mesh,
        PointerField::Pointer(SerializedAssetPointer {
            file_id: 0,
            guid: None,
            type_hint: None,
        })
    );
}

#[test]
fn preserves_shared_references_for_distinct_owners() {
    let scene = shared_reference_scene();

    let references = extract_scene_render_references(scene.as_bytes()).unwrap();

    assert_eq!(references.mesh_filters.len(), 2);
    assert_eq!(references.mesh_filters[0].game_object_file_id, 100);
    assert_eq!(references.mesh_filters[1].game_object_file_id, 101);
    assert_eq!(
        references.mesh_filters[0].mesh,
        references.mesh_filters[1].mesh
    );
    assert_eq!(references.mesh_renderers[0].game_object_file_id, 100);
    assert_eq!(references.mesh_renderers[1].game_object_file_id, 101);
}

#[test]
fn rejects_invalid_pointer_numbers_and_unknown_fields() {
    for pointer in [
        "{fileID: \"700\"}",
        "{fileID: 9223372036854775808}",
        "{fileID: 700, other: 1}",
    ] {
        let error = extract_scene_render_references(scene(Some(pointer), None).as_bytes())
            .expect_err("unsupported PPtr must fail");

        assert!(error.to_string().contains("MeshFilter.m_Mesh"));
    }

    let malformed_decimal =
        extract_scene_render_references(scene(Some("{fileID: 7x}"), None).as_bytes())
            .expect_err("malformed unquoted decimal must fail");
    assert!(
        malformed_decimal
            .to_string()
            .contains("MeshFilter.m_Mesh (document 200).fileID")
    );
    assert!(
        malformed_decimal
            .to_string()
            .contains("decimal integer syntax")
    );
}

#[test]
fn retains_negative_i64_minimum_and_type_hint_values() {
    for (wire_id, expected_id, type_hint) in
        [("-19", -19, 3), ("-9223372036854775808", i64::MIN, -7)]
    {
        let scene_text = scene(
            Some(&format!("{{fileID: {wire_id}, type: {type_hint}}}")),
            None,
        );
        let references = extract_scene_render_references(scene_text.as_bytes()).unwrap();

        assert_eq!(
            references.mesh_filters[0].mesh,
            PointerField::Pointer(SerializedAssetPointer {
                file_id: expected_id,
                guid: None,
                type_hint: Some(type_hint),
            })
        );
    }
}

#[test]
fn rejects_duplicate_dangling_unlisted_and_mismatched_owners() {
    let base = scene(Some("{fileID: 700}"), None);
    let duplicate = base.replacen(
        "  - component: {fileID: 300}",
        "  - component: {fileID: 300}\n  - component: {fileID: 300}",
        1,
    );
    let dangling = base.replacen(
        "  - component: {fileID: 300}",
        "  - component: {fileID: 999}",
        1,
    );
    let unlisted = base.replacen("  - component: {fileID: 200}\n", "", 1);
    let mismatch = base.replacen(
        "m_GameObject: {fileID: 100}",
        "m_GameObject: {fileID: 101}",
        1,
    );

    let duplicate_error = extract_scene_render_references(duplicate.as_bytes())
        .expect_err("a GameObject cannot list the same component twice");
    assert!(duplicate_error.to_string().contains(
        "GameObject document 100 lists classID 23 component document 300 more than once"
    ));

    for invalid_scene in [dangling, unlisted, mismatch] {
        assert!(extract_scene_render_references(invalid_scene.as_bytes()).is_err());
    }
}

#[test]
fn rejects_duplicate_document_ids_and_malformed_owner_lists() {
    let base = scene(Some("{fileID: 700}"), None);
    let duplicate_document = base.replacen("--- !u!23 &300", "--- !u!23 &200", 1);
    let quoted_owner_id = base.replacen(
        "component: {fileID: 200}",
        "component: {fileID: \"200\"}",
        1,
    );
    let wrong_list_entry = base.replacen("component: {fileID: 200}", "other: {fileID: 200}", 1);
    let valid_component_list = concat!(
        "  m_Component:\n",
        "  - component: {fileID: 200}\n",
        "  - component: {fileID: 300}\n",
    );
    let wrong_list_shape = base.replacen(valid_component_list, "  m_Component: null\n", 1);

    for invalid_scene in [
        duplicate_document,
        quoted_owner_id,
        wrong_list_entry,
        wrong_list_shape,
    ] {
        assert!(extract_scene_render_references(invalid_scene.as_bytes()).is_err());
    }
}

#[test]
fn rejects_a_game_object_without_a_component_list() {
    let base = scene(Some("{fileID: 700}"), None);
    let missing_owner_list = base.replacen(
        "  m_Component:\n  - component: {fileID: 200}\n  - component: {fileID: 300}\n",
        "  m_Name: Root\n",
        1,
    );

    let error = extract_scene_render_references(missing_owner_list.as_bytes())
        .expect_err("every GameObject requires a component list");

    assert!(error.to_string().contains("GameObject document 100"));
    assert!(error.to_string().contains("is missing m_Component"));
}

#[test]
fn rejects_a_render_component_listed_by_multiple_game_objects() {
    let base = scene(Some("{fileID: 700}"), None);
    let multiply_listed = base.replacen(
        "--- !u!33 &200",
        "--- !u!1 &101\nGameObject:\n  m_Component:\n  - component: {fileID: 300}\n--- !u!33 &200",
        1,
    );

    let error = extract_scene_render_references(multiply_listed.as_bytes())
        .expect_err("a render component cannot have multiple listed owners");

    assert!(
        error
            .to_string()
            .contains("classID 23 component document 300")
    );
    assert!(
        error
            .to_string()
            .contains("is listed by GameObject documents 100 and 101")
    );
}

#[test]
fn rejects_quoted_nulls_and_wrong_component_top_mapping() {
    let quoted_mesh_null = scene(Some("\"null\""), None);
    let quoted_material_null = scene(None, Some(" \"null\""));
    let scalar_null_material_slot = scene(None, Some("\n    - null"));
    let wrong_top_mapping =
        scene(Some("{fileID: 700}"), None).replacen("MeshFilter:\n", "NotMeshFilter:\n", 1);

    for invalid_scene in [
        quoted_mesh_null,
        quoted_material_null,
        scalar_null_material_slot,
        wrong_top_mapping,
    ] {
        assert!(extract_scene_render_references(invalid_scene.as_bytes()).is_err());
    }
}

#[test]
fn late_malformed_renderer_fails_the_entire_extraction() {
    let invalid_scene = scene(Some("{fileID: 700}"), Some("\n    - {fileID: \"701\"}"));

    let error = extract_scene_render_references(invalid_scene.as_bytes())
        .expect_err("late invalid renderer must prevent a partial result");

    assert!(error.to_string().contains("MeshRenderer.m_Materials"));
}

fn scene(mesh: Option<&str>, materials: Option<&str>) -> String {
    let mesh_field = mesh
        .map(|value| format!("  m_Mesh: {value}\n"))
        .unwrap_or_default();
    let material_field = materials
        .map(|value| format!("  m_Materials:{value}\n"))
        .unwrap_or_default();

    let scene_prefix = concat!(
        "--- !u!1 &100\n",
        "GameObject:\n",
        "  m_Component:\n",
        "  - component: {fileID: 200}\n",
        "  - component: {fileID: 300}\n",
        "--- !u!33 &200\n",
        "MeshFilter:\n",
        "  m_GameObject: {fileID: 100}\n",
    );
    let scene_suffix = concat!(
        "--- !u!23 &300\n",
        "MeshRenderer:\n",
        "  m_GameObject: {fileID: 100}\n",
    );

    format!("{HEADER}{scene_prefix}{mesh_field}{scene_suffix}{material_field}")
}

fn shared_reference_scene() -> String {
    format!(
        "{HEADER}{}",
        concat!(
            "--- !u!1 &100\n",
            "GameObject:\n",
            "  m_Component:\n",
            "  - component: {fileID: 200}\n",
            "  - component: {fileID: 300}\n",
            "--- !u!1 &101\n",
            "GameObject:\n",
            "  m_Component:\n",
            "  - component: {fileID: 201}\n",
            "  - component: {fileID: 301}\n",
            "--- !u!33 &200\n",
            "MeshFilter:\n",
            "  m_GameObject: {fileID: 100}\n",
            "  m_Mesh: {fileID: 700, guid: Shared-GUID}\n",
            "--- !u!23 &300\n",
            "MeshRenderer:\n",
            "  m_GameObject: {fileID: 100}\n",
            "  m_Materials: []\n",
            "--- !u!33 &201\n",
            "MeshFilter:\n",
            "  m_GameObject: {fileID: 101}\n",
            "  m_Mesh: {fileID: 700, guid: Shared-GUID}\n",
            "--- !u!23 &301\n",
            "MeshRenderer:\n",
            "  m_GameObject: {fileID: 101}\n",
            "  m_Materials: []\n",
        )
    )
}

const HEADER: &str = "%UNITY 7.0\n%YAML 1.1\n%TAG !u! tag:unity3d.com,2011:\n";
