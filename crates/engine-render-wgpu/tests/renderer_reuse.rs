//! Renderer target-resize and ordinary teardown conformance.

#![cfg(feature = "infra-render-001")]

use engine_assets::RenderAssets;
use engine_math::Affine3x4;
use engine_render_api::{
    AdapterPolicy, DrawInstance, OrthographicCamera, RenderBackend, RenderFrame, TargetSize,
};
use engine_render_wgpu::WgpuRenderer;

const ASSETS: &[u8] = br#"{
    "schema_version": 1,
    "meshes": [{
        "id": 1,
        "positions": [[-1, -1, 0], [1, -1, 0], [1, 1, 0], [-1, 1, 0]],
        "indices": [0, 1, 2, 0, 2, 3]
    }],
    "materials": [{ "id": 1, "rgba8": [255, 0, 0, 255] }]
}"#;

fn frame<'assets>(
    assets: &'assets RenderAssets,
    frame_number: u64,
    width: u32,
    height: u32,
) -> RenderFrame<'assets> {
    let target = TargetSize::try_new(width, height).unwrap();
    let camera_transform = Affine3x4::from_rows([
        [1.0, 0.0, 0.0, 0.0],
        [0.0, 1.0, 0.0, 0.0],
        [0.0, 0.0, 1.0, 0.0],
    ])
    .unwrap();
    let model_transform = Affine3x4::from_rows([
        [1.0, 0.0, 0.0, 0.0],
        [0.0, 1.0, 0.0, 0.0],
        [0.0, 0.0, 1.0, 3.0],
    ])
    .unwrap();
    let camera = OrthographicCamera::try_new_with_scale_chain(
        camera_transform,
        true,
        2.0,
        1.0,
        9.0,
        [0, 0, 0, 255],
    )
    .unwrap();
    let draw = DrawInstance::try_new(0, 0, model_transform, true, assets).unwrap();

    RenderFrame::try_new(frame_number, target, camera, assets, [draw]).unwrap()
}

#[test]
fn one_renderer_reuses_different_target_sizes_then_drops() {
    let assets = RenderAssets::parse_json(ASSETS).unwrap();
    let mut renderer = WgpuRenderer::create(AdapterPolicy::Hardware)
        .unwrap_or_else(|error| panic!("required hardware adapter unavailable: {error:?}"));

    for (frame_number, width, height) in [(1, 64, 64), (2, 32, 24)] {
        let frame = frame(&assets, frame_number, width, height);
        let image = renderer.render(&frame).unwrap();
        let receipt = image.receipt();

        assert_eq!(image.size().width(), width);
        assert_eq!(image.size().height(), height);
        assert_eq!(image.rgba8().len(), (width * height * 4) as usize);
        assert_eq!(receipt.requested_policy(), AdapterPolicy::Hardware);
        assert_eq!(receipt.target_size().width(), width);
        assert_eq!(receipt.target_size().height(), height);
        assert_eq!(receipt.frame_number(), frame_number);
        assert_eq!(receipt.draw_count(), 1);
        assert_eq!(receipt.triangle_count(), 2);
        assert!(receipt.readback_complete());

        let center_offset = ((height as usize / 2) * width as usize + width as usize / 2) * 4;
        let center: [u8; 4] = image.rgba8()[center_offset..center_offset + 4]
            .try_into()
            .unwrap();
        assert_eq!(center, [255, 0, 0, 255]);
        assert_eq!(&image.rgba8()[0..4], &[0, 0, 0, 255]);
        eprintln!(
            "reuse frame={frame_number} size={width}x{height} adapter={} backend={} center={center:?}",
            receipt.adapter().adapter_name(),
            receipt.adapter().backend()
        );
    }

    drop(renderer);
}
