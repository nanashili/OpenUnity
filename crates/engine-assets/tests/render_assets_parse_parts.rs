//! Public conformance checks for bounded borrowed render-asset fragments.

use engine_assets::{AssetError, RenderAssets};
use serde::Deserialize;
use serde_json::value::RawValue;
use serde_json::{Value, json};

#[derive(Deserialize)]
struct ManifestSlices<'a> {
    schema_version: u32,
    #[serde(borrow)]
    meshes: &'a RawValue,
    #[serde(borrow)]
    materials: &'a RawValue,
}

const MANIFEST: &[u8] = br#"{
    "schema_version": 1,
    "meshes": [{"id": 11, "positions": [[0,0,0],[1,0,0],[0,1,0]], "indices": [0,1,2]},
               {"id": 12, "positions": [[0,0,0],[2,0,0],[0,2,0]], "indices": [0,1,2]}],
    "materials": [{"id": 21, "rgba8": [255,0,0,255]}],
    "camera": {"owner": 1},
    "draws": []
}"#;

#[test]
fn borrowed_array_fragments_preserve_order_and_values() {
    let wire: ManifestSlices<'_> = serde_json::from_slice(MANIFEST).unwrap();
    let assets = RenderAssets::parse_parts(
        wire.schema_version,
        wire.meshes.get().as_bytes(),
        wire.materials.get().as_bytes(),
    )
    .unwrap();

    assert_eq!(assets.schema_version(), 1);
    assert_eq!(
        assets
            .meshes()
            .iter()
            .map(|mesh| mesh.id())
            .collect::<Vec<_>>(),
        [11, 12]
    );
    assert_eq!(assets.meshes()[0].positions()[1], [1.0, 0.0, 0.0]);
    assert_eq!(assets.meshes()[1].indices(), &[0, 1, 2]);
    assert_eq!(assets.materials()[0].id(), 21);
    assert_eq!(assets.materials()[0].rgba8(), [255, 0, 0, 255]);
}

#[test]
fn parse_json_keeps_the_existing_full_asset_document_path() {
    let assets =
        RenderAssets::parse_json(br#"{"schema_version":1,"meshes":[],"materials":[]}"#).unwrap();

    assert_eq!(assets.schema_version(), 1);
    assert!(assets.meshes().is_empty());
    assert!(assets.materials().is_empty());
}

#[test]
fn fragments_retain_duplicate_key_and_schema_validation() {
    let duplicate_mesh =
        br#"[{"id":1,"id":2,"positions":[[0,0,0],[1,0,0],[0,1,0]],"indices":[0,1,2]}]"#;
    let unknown_mesh =
        br#"[{"id":1,"positions":[[0,0,0],[1,0,0],[0,1,0]],"indices":[0,1,2],"extra":true}]"#;
    assert!(matches!(
        RenderAssets::parse_parts(1, duplicate_mesh, b"[]"),
        Err(AssetError::DuplicateKey)
    ));
    assert!(matches!(
        RenderAssets::parse_parts(1, unknown_mesh, b"[]"),
        Err(AssetError::Parse(_))
    ));
    assert!(matches!(
        RenderAssets::parse_parts(2, b"[]", b"[]"),
        Err(AssetError::Parse(_))
    ));
}

#[test]
fn fragment_path_reuses_asset_identity_and_reference_validation() {
    let duplicate_ids = br#"[
        {"id":7,"positions":[[0,0,0],[1,0,0],[0,1,0]],"indices":[0,1,2]},
        {"id":7,"positions":[[0,0,0],[1,0,0],[0,1,0]],"indices":[0,1,2]}
    ]"#;
    let invalid_index = br#"[{"id":8,"positions":[[0,0,0],[1,0,0],[0,1,0]],"indices":[0,1,3]}]"#;
    let translucent_material = br#"[{"id":9,"rgba8":[255,0,0,254]}]"#;

    assert!(matches!(
        RenderAssets::parse_parts(1, duplicate_ids, b"[]"),
        Err(AssetError::DuplicateId("mesh"))
    ));
    assert!(matches!(
        RenderAssets::parse_parts(1, invalid_index, b"[]"),
        Err(AssetError::Reference("mesh index"))
    ));
    assert!(matches!(
        RenderAssets::parse_parts(1, b"[]", translucent_material),
        Err(AssetError::Numeric("material alpha"))
    ));
}

#[test]
fn combined_fragment_bytes_are_bounded_before_parsing() {
    let oversized_fragment = vec![b' '; 16 * 1024 * 1024];

    assert!(matches!(
        RenderAssets::parse_parts(1, b"[]", &oversized_fragment),
        Err(AssetError::Limit("JSON input"))
    ));
}

#[test]
fn fragment_depth_accounts_for_the_enclosing_manifest_object() {
    let mut nested = String::from("0");
    for _ in 0..62 {
        nested = format!("[{nested}]");
    }
    let materials = format!("[{{\"id\":20,\"rgba8\":[0,0,0,255],\"unknown\":{nested}}}]");

    assert!(matches!(
        RenderAssets::parse_parts(1, b"[]", materials.as_bytes()),
        Err(AssetError::Limit("JSON depth"))
    ));
}

#[test]
fn exact_aggregate_geometry_limits_are_accepted() {
    let specifications = vec![
        (1, 65_536, 196_608),
        (2, 65_536, 196_608),
        (3, 65_536, 196_608),
        (4, 65_536, 196_608),
    ];
    let meshes = encoded_meshes(&specifications);
    let assets = RenderAssets::parse_parts(1, &meshes, b"[]").unwrap();

    assert_eq!(assets.meshes().len(), 4);
    assert_eq!(assets.meshes()[3].positions().len(), 65_536);
    assert_eq!(assets.meshes()[3].indices().len(), 196_608);
}

#[test]
fn aggregate_geometry_limits_reject_the_smallest_valid_overflow() {
    let position_overflow = encoded_meshes(&[
        (1, 65_536, 3),
        (2, 65_536, 3),
        (3, 65_536, 3),
        (4, 65_534, 3),
        (5, 3, 3),
    ]);
    assert!(matches!(
        RenderAssets::parse_parts(1, &position_overflow, b"[]"),
        Err(AssetError::Limit("mesh positions"))
    ));

    let index_overflow = encoded_meshes(&[
        (1, 3, 196_608),
        (2, 3, 196_608),
        (3, 3, 196_608),
        (4, 3, 196_608),
        (5, 3, 3),
    ]);
    assert!(matches!(
        RenderAssets::parse_parts(1, &index_overflow, b"[]"),
        Err(AssetError::Limit("mesh indices"))
    ));
}

fn encoded_meshes(specifications: &[(u32, usize, usize)]) -> Vec<u8> {
    let meshes: Vec<Value> = specifications
        .iter()
        .map(|(id, position_count, index_count)| {
            let positions = vec![[0.0, 0.0, 0.0]; *position_count];
            let indices = (0..*index_count)
                .map(|index| (index % 3) as u32)
                .collect::<Vec<_>>();
            json!({"id": id, "positions": positions, "indices": indices})
        })
        .collect();

    serde_json::to_vec(&meshes).unwrap()
}
