//! Assertions and evidence capture shared by the rendering conformance cases.

use engine_render_api::{AdapterPolicy, RenderedImage, TargetFormat};
use serde_json::Value;

pub(crate) fn assert_receipt_and_pixels(case_id: &str, image: &RenderedImage, value: &Value) {
    let receipt = image.receipt();
    assert_eq!(receipt.requested_policy(), AdapterPolicy::Hardware);
    assert!(!receipt.adapter().backend().is_empty());
    assert!(!receipt.adapter().adapter_name().is_empty());
    assert_eq!(receipt.target_format(), TargetFormat::Rgba8Unorm);
    assert_eq!(receipt.target_size().width(), 64);
    assert_eq!(receipt.target_size().height(), 64);
    assert_eq!(receipt.frame_number(), 0);
    let draws = value["draws"].as_array().unwrap();
    let enabled_draws = draws.iter().filter(|draw| draw["enabled"] != false).count();
    let triangles = value["quad_indices"].as_array().unwrap().len() / 3;
    assert_eq!(receipt.draw_count(), enabled_draws);
    assert_eq!(receipt.triangle_count(), enabled_draws * triangles);
    assert!(receipt.readback_complete());

    let expected_bytes = value
        .get("expected_rgba_bytes")
        .and_then(Value::as_u64)
        .unwrap_or(64 * 64 * 4);
    assert_eq!(image.rgba8().len() as u64, expected_bytes);
    let rgb_tolerance = value["unorm_channel_tolerance"].as_u64().unwrap() as u8;
    let alpha_tolerance = value["alpha_tolerance"].as_u64().unwrap() as u8;
    for pixel in value["pixels"].as_array().unwrap() {
        assert_pixel(image, pixel, rgb_tolerance, alpha_tolerance);
    }
    eprintln!(
        "{case_id}: adapter={} backend={} draws={} triangles={} sample={:?}",
        receipt.adapter().adapter_name(),
        receipt.adapter().backend(),
        receipt.draw_count(),
        receipt.triangle_count(),
        image.rgba8().get(0..4).unwrap()
    );
}

fn assert_pixel(image: &RenderedImage, pixel: &Value, rgb_tolerance: u8, alpha_tolerance: u8) {
    let at = pixel["at"].as_array().unwrap();
    let x = at[0].as_u64().unwrap() as usize;
    let y = at[1].as_u64().unwrap() as usize;
    let expected: [u8; 4] = pixel["rgba8"]
        .as_array()
        .unwrap()
        .iter()
        .map(|value| value.as_u64().unwrap() as u8)
        .collect::<Vec<_>>()
        .try_into()
        .unwrap();
    let offset = (y * image.size().width() as usize + x) * 4;
    let actual: [u8; 4] = image.rgba8()[offset..offset + 4].try_into().unwrap();
    eprintln!("sample ({x},{y}) actual={actual:?} expected={expected:?}");
    assert_eq!(expected[3], 255, "fixture alpha must be 255");
    assert!((i16::from(actual[3]) - i16::from(expected[3])).abs() <= i16::from(alpha_tolerance));
    for (actual, expected) in actual[..3].iter().zip(expected[..3].iter()) {
        assert!((i16::from(*actual) - i16::from(*expected)).abs() <= i16::from(rgb_tolerance));
    }
}
