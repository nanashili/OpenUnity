//! Hardware-backed public consumer coverage for the bounded OBJ importer.

#![cfg(feature = "infra-render-001")]

use engine_assets::{RenderAssets, UnlitMaterial, parse_obj_mesh};
use engine_math::Affine3x4;
use engine_render_api::{
    AdapterPolicy, DrawInstance, OrthographicCamera, RenderBackend, RenderFrame, RenderedImage,
    TargetSize,
};
use engine_render_wgpu::WgpuRenderer;

#[test]
fn imported_obj_mesh_reaches_the_hardware_renderer() {
    let mesh = parse_obj_mesh(1, b"v -1.8 -1.8 0\nv 1.8 -1.8 0\nv 0 1.8 0\nf 1 2 3\n")
        .expect("approved OBJ triangle should parse");
    let material =
        UnlitMaterial::try_new(2, [255, 0, 0, 255]).expect("opaque material should validate");
    let assets = RenderAssets::try_new(vec![mesh], vec![material])
        .expect("typed render assets should validate");

    let model = Affine3x4::from_trs([0.0, 0.0, 3.0], [0.0, 0.0, 0.0, 1.0], [1.0; 3])
        .expect("model transform should be finite");
    let camera_transform = Affine3x4::from_trs([0.0; 3], [0.0, 0.0, 0.0, 1.0], [1.0; 3])
        .expect("camera transform should be finite");
    let camera = OrthographicCamera::try_new_with_scale_chain(
        camera_transform,
        true,
        2.0,
        1.0,
        9.0,
        [0, 0, 0, 255],
    )
    .expect("camera should validate");
    let target = TargetSize::try_new(64, 64).expect("test target should validate");
    let draw = DrawInstance::try_new(0, 0, model, true, &assets)
        .expect("imported mesh and material should bind");
    let frame =
        RenderFrame::try_new(0, target, camera, &assets, [draw]).expect("frame should validate");

    let mut renderer = WgpuRenderer::create(AdapterPolicy::Hardware)
        .unwrap_or_else(|error| panic!("required hardware adapter unavailable: {error:?}"));
    let image = renderer
        .render(&frame)
        .expect("hardware renderer should produce an image");

    let receipt = image.receipt();
    assert_eq!(receipt.requested_policy(), AdapterPolicy::Hardware);
    assert!(!receipt.adapter().adapter_name().is_empty());
    assert_eq!(receipt.draw_count(), 1);
    assert_eq!(receipt.triangle_count(), 1);
    assert!(receipt.readback_complete());
    assert_eq!(pixel(&image, 32, 32), [255, 0, 0, 255]);
    assert_eq!(pixel(&image, 2, 2), [0, 0, 0, 255]);
}

fn pixel(image: &RenderedImage, x: u32, y: u32) -> [u8; 4] {
    let width = image.size().width() as usize;
    let offset = (y as usize * width + x as usize) * 4;
    image.rgba8()[offset..offset + 4]
        .try_into()
        .expect("pixel range should contain four channels")
}
