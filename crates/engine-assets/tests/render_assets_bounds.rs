//! Admission boundaries for independently authored render asset manifests.

use engine_assets::{AssetError, MeshAsset, RenderAssets, UnlitMaterial};
use serde_json::json;

const MAX_INPUT_BYTES: usize = 16 * 1024 * 1024;

#[test]
fn whole_document_accepts_exact_byte_budget_and_rejects_one_more() {
    let mut bytes = br#"{"schema_version":1,"meshes":[],"materials":[]}"#.to_vec();
    bytes.resize(MAX_INPUT_BYTES, b' ');

    assert!(RenderAssets::parse_json(&bytes).is_ok());

    bytes.push(b' ');
    assert_eq!(
        RenderAssets::parse_json(&bytes).unwrap_err(),
        AssetError::Limit("JSON input")
    );
}

#[test]
fn whole_document_depth_admission_precedes_schema_rejection() {
    // The unsupported property cannot produce a valid asset, but distinguishes
    // the depth admission limit from subsequent schema validation.
    let at_limit = nested_unknown_property(64);
    assert!(matches!(
        RenderAssets::parse_json(&at_limit),
        Err(AssetError::Parse(_))
    ));

    let beyond_limit = nested_unknown_property(65);
    assert_eq!(
        RenderAssets::parse_json(&beyond_limit).unwrap_err(),
        AssetError::Limit("JSON depth")
    );
}

#[test]
fn mesh_collection_accepts_256_entries_and_rejects_257() {
    let mut meshes = (1..=256).map(encoded_mesh).collect::<Vec<_>>();
    let at_limit = encode_manifest(&meshes);
    assert_eq!(
        RenderAssets::parse_json(&at_limit).unwrap().meshes().len(),
        256
    );

    meshes.push(encoded_mesh(257));
    assert_eq!(
        RenderAssets::parse_json(&encode_manifest(&meshes)).unwrap_err(),
        AssetError::Limit("render asset count")
    );
}

#[test]
fn typed_geometry_rejects_invalid_ids_lengths_numbers_and_references() {
    let positions = vec![[0.0; 3]; 3];
    for id in [0, 65_536] {
        assert_eq!(
            MeshAsset::try_new(id, positions.clone(), vec![0, 1, 2]).unwrap_err(),
            AssetError::Limit("mesh ID")
        );
    }

    assert_eq!(
        MeshAsset::try_new(1, vec![[0.0; 3]; 2], vec![0, 1, 0]).unwrap_err(),
        AssetError::Limit("mesh positions")
    );
    for indices in [vec![], vec![0, 1], vec![0, 1, 2, 0]] {
        assert_eq!(
            MeshAsset::try_new(1, positions.clone(), indices).unwrap_err(),
            AssetError::Limit("mesh indices")
        );
    }

    for number in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
        let mut invalid_positions = positions.clone();
        invalid_positions[1][2] = number;
        assert_eq!(
            MeshAsset::try_new(1, invalid_positions, vec![0, 1, 2]).unwrap_err(),
            AssetError::Numeric("mesh position")
        );
    }

    assert_eq!(
        MeshAsset::try_new(1, positions, vec![0, 1, 3]).unwrap_err(),
        AssetError::Reference("mesh index")
    );
}

#[test]
fn typed_material_rejects_invalid_ids_and_transparency() {
    for id in [0, 65_536] {
        assert_eq!(
            UnlitMaterial::try_new(id, [0, 0, 0, 255]).unwrap_err(),
            AssetError::Limit("material ID")
        );
    }

    assert_eq!(
        UnlitMaterial::try_new(1, [0, 0, 0, 254]).unwrap_err(),
        AssetError::Numeric("material alpha")
    );
}

fn nested_unknown_property(depth: usize) -> Vec<u8> {
    let arrays = depth - 1;
    format!(
        "{{\"schema_version\":1,\"meshes\":[],\"materials\":[],\"unknown\":{}0{}}}",
        "[".repeat(arrays),
        "]".repeat(arrays)
    )
    .into_bytes()
}

fn encoded_mesh(id: u32) -> serde_json::Value {
    json!({"id": id, "positions": [[0,0,0],[1,0,0],[0,1,0]], "indices": [0,1,2]})
}

fn encode_manifest(meshes: &[serde_json::Value]) -> Vec<u8> {
    serde_json::to_vec(&json!({"schema_version":1,"meshes":meshes,"materials":[]})).unwrap()
}
