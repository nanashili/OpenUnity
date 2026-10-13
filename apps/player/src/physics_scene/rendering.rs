use engine_math::{Affine3x4, Quaternion, from_rotation_vector, multiply};
use engine_render_api::{DrawInstance, OrthographicCamera, RenderBackend, RenderFrame, TargetSize};
use engine_render_wgpu::WgpuRenderer;

use crate::render_output::{CallbackResult, RenderOutputPublisher};

use super::arguments::Dimension;
use super::error::PhysicsSceneError;
use super::scene::RuntimeScene;
use super::simulation::SimulationReport;
use super::snapshot::{CaptureMoment, SceneSnapshot, capture_scene};

const CAMERA_NEAR: f32 = 0.1;
const CAMERA_FAR_2D: f32 = 10.0;
const CAMERA_FAR_3D: f32 = 15.0;
const CAMERA_HALF_HEIGHT_2D: f32 = 3.0;
const CAMERA_HALF_HEIGHT_3D: f32 = 4.2;
const CAMERA_CLEAR_RGBA8: [u8; 4] = [24, 30, 42, 255];

pub(super) fn render_timeline(
    report: &SimulationReport,
    renderer: &mut WgpuRenderer,
    assets: &engine_assets::RenderAssets,
    target_size: TargetSize,
    publisher: &mut RenderOutputPublisher,
) -> Result<(), PhysicsSceneError> {
    for (ordinal, snapshot) in report.snapshots.iter().enumerate() {
        let frame = build_frame(snapshot, report.dimension, assets, target_size)?;
        let image = renderer
            .render(&frame)
            .map_err(|error| PhysicsSceneError::debug("render physics-scene snapshot", error))?;

        if image.receipt().frame_number() != frame.frame_number() {
            return Err(PhysicsSceneError::new(
                "renderer returned a different physics-scene frame number",
            ));
        }

        publisher
            .publish_frame(
                u64::try_from(ordinal)
                    .map_err(|_| PhysicsSceneError::new("frame ordinal exceeds u64"))?,
                &image,
                CallbackResult::NotApplicable,
            )
            .map_err(|error| PhysicsSceneError::display("publish physics-scene frame", error))?;
    }

    Ok(())
}

fn build_frame<'assets>(
    snapshot: &SceneSnapshot,
    dimension: Dimension,
    assets: &'assets engine_assets::RenderAssets,
    target_size: TargetSize,
) -> Result<RenderFrame<'assets>, PhysicsSceneError> {
    let frame_number = u64::try_from(snapshot.simulation_step)
        .map_err(|_| PhysicsSceneError::new("simulation step exceeds frame-number range"))?;

    build_frame_with_number(snapshot, dimension, assets, target_size, frame_number)
}

/// Build one live frame from the current runtime Transform and solver state.
pub(crate) fn build_live_frame<'assets>(
    scene: &RuntimeScene,
    assets: &'assets engine_assets::RenderAssets,
    target_size: TargetSize,
    simulation_step: usize,
    frame_number: u64,
) -> Result<RenderFrame<'assets>, PhysicsSceneError> {
    let snapshot = capture_scene(scene, CaptureMoment::Live, simulation_step)?;

    build_frame_with_number(
        &snapshot,
        scene.dimension(),
        assets,
        target_size,
        frame_number,
    )
}

fn build_frame_with_number<'assets>(
    snapshot: &SceneSnapshot,
    dimension: Dimension,
    assets: &'assets engine_assets::RenderAssets,
    target_size: TargetSize,
    frame_number: u64,
) -> Result<RenderFrame<'assets>, PhysicsSceneError> {
    let camera = create_camera(dimension)?;

    let draws = snapshot
        .bodies
        .iter()
        .map(|body| build_draw(*body, assets))
        .collect::<Result<Vec<_>, _>>()?;
    RenderFrame::try_new(frame_number, target_size, camera, assets, draws)
        .map_err(|error| PhysicsSceneError::debug("validate physics-scene frame", error))
}

fn create_camera(dimension: Dimension) -> Result<OrthographicCamera, PhysicsSceneError> {
    let (position, rotation, half_height, far) = match dimension {
        Dimension::TwoD => (
            [0.0, 0.0, 0.0],
            [0.0, 0.0, 0.0, 1.0],
            CAMERA_HALF_HEIGHT_2D,
            CAMERA_FAR_2D,
        ),
        Dimension::ThreeD => (
            [4.0, 3.0, -3.0],
            three_dimensional_camera_rotation()?,
            CAMERA_HALF_HEIGHT_3D,
            CAMERA_FAR_3D,
        ),
    };
    let world_from_camera = Affine3x4::from_trs(position, rotation, [1.0, 1.0, 1.0])
        .map_err(|error| PhysicsSceneError::debug("create physics-scene camera pose", error))?;

    OrthographicCamera::try_new_with_scale_chain(
        world_from_camera,
        true,
        half_height,
        CAMERA_NEAR,
        far,
        CAMERA_CLEAR_RGBA8,
    )
    .map_err(|error| PhysicsSceneError::debug("validate physics-scene camera", error))
}

fn three_dimensional_camera_rotation() -> Result<Quaternion, PhysicsSceneError> {
    let pitch = from_rotation_vector([0.4, 0.0, 0.0])
        .map_err(|error| PhysicsSceneError::debug("create physics-scene camera pitch", error))?;
    let yaw = from_rotation_vector([0.0, -0.6, 0.0])
        .map_err(|error| PhysicsSceneError::debug("create physics-scene camera yaw", error))?;

    multiply(yaw, pitch)
        .map_err(|error| PhysicsSceneError::debug("compose physics-scene camera rotation", error))
}

fn build_draw(
    body: super::snapshot::BodySnapshot,
    assets: &engine_assets::RenderAssets,
) -> Result<DrawInstance, PhysicsSceneError> {
    let scale =
        std::array::from_fn(|axis| body.transform.scale[axis] * body.collider_half_extents[axis]);
    let affine = Affine3x4::from_trs(body.transform.position, body.transform.rotation, scale)
        .map_err(|error| PhysicsSceneError::debug("convert runtime Transform snapshot", error))?;

    DrawInstance::try_new(body.mesh_index, body.material_index, affine, true, assets)
        .map_err(|error| PhysicsSceneError::debug("validate physics-scene draw", error))
}
