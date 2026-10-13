use engine_assets::RenderAssets;
use engine_math::Affine3x4;
use engine_render_api::{
    AdapterPolicy, DrawInstance, OrthographicCamera, RenderBackend, RenderError, RenderFrame,
    TargetSize,
};

use crate::renderer::WgpuRenderer;

const ASSETS: &[u8] = br#"{
    "schema_version": 1,
    "meshes": [{
        "id": 1,
        "positions": [[-1, -1, 0], [1, -1, 0], [1, 1, 0], [-1, 1, 0]],
        "indices": [0, 1, 2, 0, 2, 3]
    }],
    "materials": [{ "id": 1, "rgba8": [255, 0, 0, 255] }]
}"#;

fn device_loss_frame<'assets>(
    assets: &'assets RenderAssets,
    overflow: bool,
) -> RenderFrame<'assets> {
    let target = TargetSize::try_new(32, 32).unwrap();
    let identity = Affine3x4::from_rows([
        [1.0, 0.0, 0.0, 0.0],
        [0.0, 1.0, 0.0, 0.0],
        [0.0, 0.0, 1.0, 0.0],
    ])
    .unwrap();
    let model_scale = if overflow { f32::MAX } else { 1.0 };
    let model_offset = if overflow { f32::MAX } else { 0.0 };
    let model = Affine3x4::from_rows([
        [model_scale, 0.0, 0.0, model_offset],
        [0.0, 1.0, 0.0, 0.0],
        [0.0, 0.0, 1.0, 3.0],
    ])
    .unwrap();
    let camera =
        OrthographicCamera::try_new_with_scale_chain(identity, true, 2.0, 1.0, 9.0, [0, 0, 0, 255])
            .unwrap();
    let draw = DrawInstance::try_new(0, 0, model, true, assets).unwrap();

    RenderFrame::try_new(1, target, camera, assets, vec![draw]).unwrap()
}

fn pixel(rgba8: &[u8], width: usize, x: usize, y: usize) -> [u8; 4] {
    let pixel_index = y.checked_mul(width).unwrap().checked_add(x).unwrap();
    let byte_offset = pixel_index.checked_mul(4).unwrap();
    rgba8[byte_offset..byte_offset + 4].try_into().unwrap()
}

#[test]
fn destroyed_device_callback_after_map_blocks_image_publication() {
    let assets = RenderAssets::parse_json(ASSETS).unwrap();
    let mut renderer = WgpuRenderer::create(AdapterPolicy::Hardware)
        .unwrap_or_else(|error| panic!("required hardware adapter unavailable: {error:?}"));
    let frame = device_loss_frame(&assets, false);
    let overflow_frame = device_loss_frame(&assets, true);
    assert!(matches!(
        renderer.validate_frame(&overflow_frame),
        Err(RenderError::Numeric)
    ));

    let (size, validated_draws) = renderer.validate_frame(&frame).unwrap();
    let (targets, readback) = renderer.allocate_frame_resources(size).unwrap();
    let (draw_count, load) = renderer
        .submit_ordered_draws(&frame, &targets, &validated_draws)
        .unwrap();
    let (rgba8, readback_complete) = renderer
        .complete_readback(&frame, &targets, &readback, size, load)
        .unwrap();

    assert_eq!(rgba8.len(), 32 * 32 * 4);
    assert!(readback_complete);
    assert_eq!(pixel(&rgba8, 32, 16, 16), [255, 0, 0, 255]);
    assert_eq!(pixel(&rgba8, 32, 0, 0), [0, 0, 0, 255]);

    let pending_submission = renderer.device.queue().submit([]);
    renderer.device.device().destroy();
    renderer
        .device
        .device()
        .poll(wgpu::PollType::Wait {
            submission_index: None,
            timeout: Some(crate::renderer::GPU_WAIT),
        })
        .expect("destroyed device must complete its bounded callback poll");
    assert!(matches!(
        renderer.device.ensure_healthy(),
        Err(crate::device::DeviceError::DeviceLost(_))
    ));

    let failed_readback = crate::readback::map_rgba8(
        renderer.device.device(),
        &readback,
        pending_submission,
        size,
        crate::renderer::GPU_WAIT,
    );
    assert!(failed_readback.is_err());
    let error = renderer
        .device
        .complete_operation(failed_readback.map_err(crate::backend_mapping::map_readback_error))
        .expect_err("readback failure must retain the recorded device loss");
    assert_eq!(
        crate::backend_mapping::map_device_error(error.clone()),
        RenderError::DeviceLost
    );
    assert!(matches!(
        error,
        crate::device::DeviceError::RecordedFailureAndOperation { .. }
    ));

    let result = renderer.publish_image(
        &frame,
        rgba8,
        draw_count,
        validated_draws,
        readback_complete,
    );
    assert!(matches!(result, Err(RenderError::DeviceLost)));
    assert!(matches!(
        renderer.render(&overflow_frame),
        Err(RenderError::DeviceLost)
    ));
    assert!(matches!(
        renderer.render(&frame),
        Err(RenderError::DeviceLost)
    ));
}
