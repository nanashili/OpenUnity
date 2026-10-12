use crate::{AssetError, RenderAssets};

fn mesh_json(id: usize, position_count: usize, index_count: usize) -> String {
    let positions = repeated_values(position_count, "[0,0,0]");
    let indices = repeated_values(index_count, "0");

    format!("{{\"id\":{id},\"positions\":[{positions}],\"indices\":[{indices}]}}")
}

fn repeated_values(count: usize, value: &str) -> String {
    let mut values = String::with_capacity(count.saturating_mul(value.len() + 1));

    for index in 0..count {
        if index > 0 {
            values.push(',');
        }
        values.push_str(value);
    }

    values
}

fn assets_json(meshes: &[String], materials: &[String]) -> String {
    format!(
        "{{\"schema_version\":1,\"meshes\":[{}],\"materials\":[{}]}}",
        meshes.join(","),
        materials.join(",")
    )
}

#[test]
fn valid_assets_parse_and_expose_validated_values() {
    let json = assets_json(
        &[mesh_json(1, 3, 3)],
        &["{\"id\":2,\"rgba8\":[1,2,3,255]}".to_owned()],
    );

    let assets = RenderAssets::parse_json(json.as_bytes()).unwrap();

    assert_eq!(assets.schema_version(), 1);
    assert_eq!(assets.meshes().len(), 1);
    assert_eq!(assets.meshes()[0].positions().len(), 3);
    assert_eq!(assets.meshes()[0].indices(), &[0, 0, 0]);
    assert_eq!(assets.materials()[0].rgba8(), [1, 2, 3, 255]);
}

#[test]
fn mesh_buffers_accept_exact_per_mesh_limit_and_reject_one_more() {
    let exact = assets_json(&[mesh_json(1, 65_536, 3)], &[]);
    let assets = RenderAssets::parse_json(exact.as_bytes()).unwrap();
    assert_eq!(assets.meshes()[0].positions().len(), 65_536);

    let oversized = assets_json(&[mesh_json(1, 65_537, 3)], &[]);
    assert_eq!(
        RenderAssets::parse_json(oversized.as_bytes()).unwrap_err(),
        AssetError::Limit("mesh positions")
    );
}

#[test]
fn aggregate_position_budget_is_enforced_during_mesh_deserialization() {
    let exact_meshes = (1..=4)
        .map(|id| mesh_json(id, 65_536, 3))
        .collect::<Vec<_>>();
    let exact = assets_json(&exact_meshes, &[]);
    let assets = RenderAssets::parse_json(exact.as_bytes()).unwrap();
    assert_eq!(
        assets
            .meshes()
            .iter()
            .map(|mesh| mesh.positions().len())
            .sum::<usize>(),
        262_144
    );

    let mut oversized_meshes = exact_meshes;
    oversized_meshes.push(mesh_json(5, 3, 3));
    let oversized = assets_json(&oversized_meshes, &[]);
    assert_eq!(
        RenderAssets::parse_json(oversized.as_bytes()).unwrap_err(),
        AssetError::Limit("mesh positions")
    );
}

#[test]
fn aggregate_index_budget_is_enforced_during_mesh_deserialization() {
    let exact_meshes = (1..=4)
        .map(|id| mesh_json(id, 3, 196_608))
        .collect::<Vec<_>>();
    let exact = assets_json(&exact_meshes, &[]);
    let assets = RenderAssets::parse_json(exact.as_bytes()).unwrap();
    assert_eq!(
        assets
            .meshes()
            .iter()
            .map(|mesh| mesh.indices().len())
            .sum::<usize>(),
        786_432
    );

    let mut oversized_meshes = exact_meshes;
    oversized_meshes.push(mesh_json(5, 3, 3));
    let oversized = assets_json(&oversized_meshes, &[]);
    assert_eq!(
        RenderAssets::parse_json(oversized.as_bytes()).unwrap_err(),
        AssetError::Limit("mesh indices")
    );
}

#[test]
fn duplicate_unknown_and_excess_material_fields_fail_closed() {
    let duplicate = r#"{"schema_version":1,"meshes":[{"id":1,"id":2,"positions":[[0,0,0],[1,0,0],[0,1,0]],"indices":[0,1,2]}],"materials":[]}"#;
    assert_eq!(
        RenderAssets::parse_json(duplicate.as_bytes()).unwrap_err(),
        AssetError::DuplicateKey
    );

    let unknown = r#"{"schema_version":1,"meshes":[],"materials":[],"other":0}"#;
    assert!(matches!(
        RenderAssets::parse_json(unknown.as_bytes()),
        Err(AssetError::Parse(_))
    ));

    let materials = (1..=257)
        .map(|id| format!("{{\"id\":{id},\"rgba8\":[0,0,0,255]}}"))
        .collect::<Vec<_>>();
    let oversized = assets_json(&[], &materials);
    assert_eq!(
        RenderAssets::parse_json(oversized.as_bytes()).unwrap_err(),
        AssetError::Limit("render asset count")
    );
}
