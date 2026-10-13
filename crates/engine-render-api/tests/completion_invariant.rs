//! Public API checks for completed readback publication.

use engine_assets::RenderAssets;
use engine_math::Affine3x4;
use engine_render_api::{
    AdapterIdentity, AdapterPolicy, BackendCompletion, BackendReceipt, OrthographicCamera,
    RenderError, RenderFrame, RenderedImage, TargetFormat, TargetSize,
};

const EMPTY_ASSETS: &[u8] = br#"{
    "schema_version": 1,
    "meshes": [],
    "materials": []
}"#;

fn frame<'assets>(assets: &'assets RenderAssets) -> RenderFrame<'assets> {
    frame_with_target(assets, 2, 1)
}

fn frame_with_target<'assets>(
    assets: &'assets RenderAssets,
    width: u32,
    height: u32,
) -> RenderFrame<'assets> {
    let target = TargetSize::try_new(width, height).expect("test target must be valid");
    let camera_transform = Affine3x4::from_rows([
        [1.0, 0.0, 0.0, 0.0],
        [0.0, 1.0, 0.0, 0.0],
        [0.0, 0.0, 1.0, 0.0],
    ])
    .expect("identity camera transform must be finite");
    let camera = OrthographicCamera::try_new_with_scale_chain(
        camera_transform,
        true,
        2.0,
        1.0,
        9.0,
        [0, 0, 0, 255],
    )
    .expect("test camera must be valid");

    RenderFrame::try_new(7, target, camera, assets, []).expect("empty frame must be valid")
}

fn receipt(frame: &RenderFrame<'_>, readback_complete: bool) -> BackendReceipt {
    BackendReceipt::new(
        AdapterPolicy::Hardware,
        AdapterIdentity::new(
            "test-backend",
            "test-adapter",
            "test-vendor",
            "test-device",
            "test-driver",
            "test-driver-info",
            "Hardware",
        ),
        TargetFormat::Rgba8Unorm,
        frame,
        BackendCompletion::new(0, 0, readback_complete),
    )
}

#[test]
fn incomplete_readback_is_rejected() {
    let assets = RenderAssets::parse_json(EMPTY_ASSETS).expect("empty assets must be valid");
    let frame = frame(&assets);
    let bytes = [10, 20, 30, 255, 40, 50, 60, 255];

    let incomplete = RenderedImage::try_new(frame.target_size(), bytes, receipt(&frame, false));
    assert!(matches!(incomplete, Err(RenderError::Readback)));
}

#[test]
fn mismatched_receipt_size_is_rejected() {
    let assets = RenderAssets::parse_json(EMPTY_ASSETS).expect("empty assets must be valid");
    let frame = frame(&assets);
    let mismatched_frame = frame_with_target(&assets, 1, 1);
    let bytes = [10, 20, 30, 255, 40, 50, 60, 255];

    let mismatched =
        RenderedImage::try_new(frame.target_size(), bytes, receipt(&mismatched_frame, true));
    assert!(matches!(mismatched, Err(RenderError::ReceiptMismatch)));
    assert!(!RenderError::ReceiptMismatch.is_device_failure());
}

#[test]
fn matching_receipt_size_preserves_image_and_receipt() {
    let assets = RenderAssets::parse_json(EMPTY_ASSETS).expect("empty assets must be valid");
    let frame = frame(&assets);
    let bytes = [10, 20, 30, 255, 40, 50, 60, 255];
    let complete_receipt = receipt(&frame, true);

    let image = RenderedImage::try_new(frame.target_size(), bytes, complete_receipt.clone())
        .expect("completed readback must publish");
    assert_eq!(image.size(), frame.target_size());
    assert_eq!(image.rgba8(), bytes);
    assert_eq!(image.receipt(), &complete_receipt);
}

#[test]
fn short_and_long_readbacks_are_rejected_as_readback_failures() {
    let assets = RenderAssets::parse_json(EMPTY_ASSETS).expect("empty assets must be valid");
    let frame = frame(&assets);
    for bytes in [vec![0; 7], vec![0; 9]] {
        let result = RenderedImage::try_new(frame.target_size(), bytes, receipt(&frame, true));
        assert!(matches!(result, Err(RenderError::Readback)));
    }
}
