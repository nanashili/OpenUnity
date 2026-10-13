//! INFRA-RENDER-001 fixture and backend conformance checks.

#![cfg(feature = "infra-render-001")]

#[path = "support/mod.rs"]
mod support;

use engine_assets::{AssetError, RenderAssets};
use engine_math::Affine3x4;
use engine_render_api::{
    AdapterPolicy, DrawInstance, OrthographicCamera, RenderBackend, RenderError, RenderFrame,
    RenderedImage, TargetSize,
};
use engine_render_wgpu::WgpuRenderer;
use serde::Deserialize;
use serde_json::{Map, Value};

const FIXTURES: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/infra_render_001.json"
));

#[derive(Debug, Deserialize)]
struct FixtureFile {
    contract: String,
    version: String,
    base: Value,
    cases: Vec<FixtureCase>,
}

#[derive(Debug, Deserialize)]
struct FixtureCase {
    id: String,
    #[serde(flatten)]
    values: Map<String, Value>,
}

fn fixture_file() -> FixtureFile {
    serde_json::from_slice(FIXTURES).expect("approved rendering fixture JSON must remain valid")
}
fn merged_case(case_id: &str) -> Value {
    let fixtures = fixture_file();
    assert_eq!(fixtures.contract, "INFRA-RENDER-001");
    assert_eq!(fixtures.version, "0.1.0");

    let mut merged = fixtures.base.as_object().unwrap().clone();
    let case = fixtures
        .cases
        .into_iter()
        .find(|case| case.id == case_id)
        .unwrap_or_else(|| panic!("approved fixture is missing case {case_id}"));
    merged.extend(case.values);
    Value::Object(merged)
}
fn numbers(value: &Value, field: &str) -> Vec<f32> {
    numbers_value(&value[field], field)
}

fn numbers_value(value: &Value, field: &str) -> Vec<f32> {
    value
        .as_array()
        .unwrap_or_else(|| panic!("fixture field {field} must be an array"))
        .iter()
        .map(|value| value.as_f64().expect("fixture number must be finite") as f32)
        .collect()
}
fn vector<const N: usize>(value: &Value, field: &str) -> [f32; N] {
    numbers(value, field)
        .try_into()
        .unwrap_or_else(|_| panic!("fixture field {field} has wrong length"))
}
fn positions(value: &Value) -> Vec<[f32; 3]> {
    value["quad_positions"]
        .as_array()
        .unwrap()
        .iter()
        .map(|row| {
            row.as_array()
                .unwrap()
                .iter()
                .map(|value| value.as_f64().unwrap() as f32)
                .collect::<Vec<_>>()
                .try_into()
                .unwrap()
        })
        .collect()
}
fn indices(value: &Value) -> Vec<u32> {
    value["quad_indices"]
        .as_array()
        .unwrap()
        .iter()
        .map(|value| value.as_u64().unwrap() as u32)
        .collect()
}
fn transform(value: &Value, prefix: &str) -> Affine3x4 {
    let position = vector::<3>(value, &format!("{prefix}position"));
    let rotation = vector::<4>(value, &format!("{prefix}rotation_xyzw"));
    let scale = vector::<3>(value, &format!("{prefix}local_scale"));
    Affine3x4::from_trs(position, rotation, scale).expect("fixture transform must be finite")
}

fn model_transform(base: &Value, draw: &Value) -> Affine3x4 {
    let field = |name: &str| draw.get(name).unwrap_or(&base[name]);
    let position = numbers_value(field("model_position"), "model_position")
        .try_into()
        .unwrap();
    let rotation = numbers_value(field("model_rotation_xyzw"), "model_rotation_xyzw")
        .try_into()
        .unwrap();
    let scale = numbers_value(field("model_local_scale"), "model_local_scale")
        .try_into()
        .unwrap();
    Affine3x4::from_trs(position, rotation, scale).expect("fixture transform must be finite")
}
fn assets(value: &Value) -> Result<RenderAssets, AssetError> {
    let manifest = serde_json::json!({
        "schema_version": 1,
        "meshes": [{
            "id": 1,
            "positions": positions(value),
            "indices": indices(value),
        }],
        "materials": [
            {"id": 2, "rgba8": [255, 0, 0, 255]},
            {"id": 3, "rgba8": [0, 0, 255, 255]},
        ],
    });
    let bytes = serde_json::to_vec(&manifest)
        .map_err(|error| AssetError::Parse(format!("asset manifest encoding failed: {error}")))?;
    RenderAssets::parse_json(&bytes)
}
fn camera(value: &Value) -> Result<OrthographicCamera, RenderError> {
    let unit_scale_chain = vector::<3>(value, "camera_local_scale") == [1.0; 3];
    OrthographicCamera::try_new_with_scale_chain(
        transform(value, "camera_"),
        unit_scale_chain,
        value["half_height"].as_f64().unwrap() as f32,
        value["near"].as_f64().unwrap() as f32,
        value["far"].as_f64().unwrap() as f32,
        value["clear_rgba8"]
            .as_array()
            .unwrap()
            .iter()
            .map(|value| value.as_u64().unwrap() as u8)
            .collect::<Vec<_>>()
            .try_into()
            .unwrap(),
    )
}
fn world_transform(value: &Value, draw: &Value) -> Result<Affine3x4, String> {
    let local = model_transform(value, draw);
    value
        .get("parent_position")
        .map(|_| transform(value, "parent_").compose(&local))
        .unwrap_or(Ok(local))
        .map_err(|error| format!("world transform composition failed: {error:?}"))
}
fn frame_with_target<'assets>(
    value: &Value,
    assets: &'assets RenderAssets,
    frame_number: u64,
    width: u32,
    height: u32,
) -> Result<RenderFrame<'assets>, String> {
    let draws = value["draws"]
        .as_array()
        .unwrap()
        .iter()
        .map(|draw| {
            let world = world_transform(value, draw)?;
            let material = usize::from(draw["material"].as_str() == Some("blue"));
            DrawInstance::try_new(0, material as u32, world, draw["enabled"] != false, assets)
                .map_err(|error| format!("draw construction failed: {error:?}"))
        })
        .collect::<Result<Vec<_>, _>>()?;
    let size =
        TargetSize::try_new(width, height).map_err(|error| format!("target failed: {error:?}"))?;
    RenderFrame::try_new(
        frame_number,
        size,
        camera(value).map_err(|error| format!("camera construction failed: {error:?}"))?,
        assets,
        draws,
    )
    .map_err(|error| format!("frame construction failed: {error:?}"))
}

fn affine_columns(transform: &Affine3x4) -> [[f32; 3]; 4] {
    let origin = transform.transform_point([0.0, 0.0, 0.0]).unwrap();
    let x_axis = transform.transform_point([1.0, 0.0, 0.0]).unwrap();
    let y_axis = transform.transform_point([0.0, 1.0, 0.0]).unwrap();
    let z_axis = transform.transform_point([0.0, 0.0, 1.0]).unwrap();
    [
        [
            x_axis[0] - origin[0],
            x_axis[1] - origin[1],
            x_axis[2] - origin[2],
        ],
        [
            y_axis[0] - origin[0],
            y_axis[1] - origin[1],
            y_axis[2] - origin[2],
        ],
        [
            z_axis[0] - origin[0],
            z_axis[1] - origin[1],
            z_axis[2] - origin[2],
        ],
        origin,
    ]
}

fn assert_world_affine(value: &Value) {
    let Some(expected) = value.get("expected_world_affine_columns") else {
        return;
    };
    let expected: Vec<[f32; 3]> = serde_json::from_value(expected.clone()).unwrap();
    let tolerance = value["affine_absolute_tolerance"].as_f64().unwrap() as f32;
    let draw = &value["draws"].as_array().unwrap()[0];
    let actual = affine_columns(&world_transform(value, draw).unwrap());
    assert_eq!(expected.len(), actual.len());
    for (column, (actual, expected)) in actual.into_iter().zip(expected).enumerate() {
        for (axis, (actual, expected)) in actual.into_iter().zip(expected).enumerate() {
            assert!(
                (actual - expected).abs() <= tolerance,
                "world affine column {column} axis {axis}: got {actual}, expected {expected}"
            );
        }
    }
}

fn clip_polygon_z(polygon: Vec<[f32; 3]>, boundary: f32, keep_greater: bool) -> Vec<[f32; 3]> {
    let mut clipped = Vec::new();
    for index in 0..polygon.len() {
        let previous = &polygon[(index + polygon.len() - 1) % polygon.len()];
        let current = &polygon[index];
        let previous_inside = if keep_greater {
            previous[2] >= boundary
        } else {
            previous[2] <= boundary
        };
        let current_inside = if keep_greater {
            current[2] >= boundary
        } else {
            current[2] <= boundary
        };
        if previous_inside != current_inside {
            let factor = (boundary - previous[2]) / (current[2] - previous[2]);
            clipped.push(std::array::from_fn(|axis| {
                previous[axis] + factor * (current[axis] - previous[axis])
            }));
        }
        if current_inside {
            clipped.push(*current);
        }
    }
    clipped
}

fn assert_clipped_polygon(value: &Value) {
    let Some(expected) = value.get("expected_clipped_camera_polygon") else {
        return;
    };
    let expected: Vec<[f32; 3]> = serde_json::from_value(expected.clone()).unwrap();
    let draw = &value["draws"].as_array().unwrap()[0];
    let world = world_transform(value, draw).unwrap();
    let camera = transform(value, "camera_").inverse().unwrap();
    let positions = positions(value);
    let indices = indices(value);
    let polygon = indices[..3]
        .iter()
        .map(|index| {
            let local = positions[*index as usize];
            let world_point = world.transform_point(local).unwrap();
            camera.transform_point(world_point).unwrap()
        })
        .collect();
    let polygon = clip_polygon_z(polygon, value["near"].as_f64().unwrap() as f32, true);
    let polygon = clip_polygon_z(polygon, value["far"].as_f64().unwrap() as f32, false);
    assert_eq!(polygon.len(), expected.len());
    let tolerance = value["affine_absolute_tolerance"]
        .as_f64()
        .unwrap_or(0.00001) as f32;
    let matches_rotation = (0..expected.len()).any(|offset| {
        polygon.iter().enumerate().all(|(index, actual)| {
            let expected_point = expected[(index + offset) % expected.len()];
            actual
                .iter()
                .zip(expected_point)
                .all(|(actual, expected)| (*actual - expected).abs() <= tolerance)
        })
    });
    assert!(
        matches_rotation,
        "clipped polygon does not match fixture oracle"
    );
}

fn render_case(case_id: &str, renderer: &mut WgpuRenderer) -> RenderedImage {
    let value = merged_case(case_id);
    let assets = assets(&value).unwrap();
    let frame = frame_with_target(&value, &assets, 0, 64, 64).unwrap();
    renderer.render(&frame).unwrap()
}

fn assert_case_pixels(case_id: &str, renderer: &mut WgpuRenderer) {
    let value = merged_case(case_id);
    assert_world_affine(&value);
    assert_clipped_polygon(&value);
    let image = render_case(case_id, renderer);
    support::assert_receipt_and_pixels(case_id, &image, &value);
}

fn backend() -> WgpuRenderer {
    WgpuRenderer::create(AdapterPolicy::Hardware)
        .unwrap_or_else(|error| panic!("required hardware adapter unavailable: {error:?}"))
}

#[test]
fn static_pixel_cases_use_real_backend_readback() {
    let mut renderer = backend();
    for case_id in [
        "solid-and-clear",
        "near-depth-first",
        "near-depth-last",
        "equal-depth-last-wins",
        "camera-translation",
        "model-reflection",
        "hierarchy-preserves-shear",
        "row-orientation",
        "clip-outside-near-and-far",
        "triangle-crosses-near",
        "triangle-crosses-far",
    ] {
        assert_case_pixels(case_id, &mut renderer);
    }
}

#[test]
fn malformed_camera_and_mesh_fail_before_backend_creation() {
    let camera_case = merged_case("unsupported-camera-scale");
    assert!(matches!(
        camera(&camera_case),
        Err(RenderError::InvalidCamera)
    ));

    let mesh_case = merged_case("invalid-mesh-index");
    assert!(matches!(
        assets(&mesh_case),
        Err(AssetError::Reference("mesh index"))
    ));
}
