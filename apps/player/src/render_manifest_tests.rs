use super::{RenderManifest, RenderManifestError};

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
