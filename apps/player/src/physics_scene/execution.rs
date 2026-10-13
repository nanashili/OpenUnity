use engine_render_wgpu::WgpuRenderer;

use crate::render_output::{CleanupOutcome, CleanupResult, PublishError, RenderOutputPublisher};

use super::arguments::Arguments;
use super::error::PhysicsSceneError;
use super::geometry;
use super::receipt::PhysicsSceneReceipt;
use super::rendering;
use super::scene::RuntimeScene;
use super::simulation::{self, SimulationReport};

pub(crate) fn run(values: &[String]) -> Result<(), PhysicsSceneError> {
    let arguments = Arguments::parse(values)?;
    let publisher =
        RenderOutputPublisher::create(&arguments.output, arguments.target_size, arguments.policy)
            .map_err(|error| PhysicsSceneError::display("admit physics-scene output", error))?;

    run_admitted(arguments, publisher)
}

fn run_admitted(
    arguments: Arguments,
    publisher: RenderOutputPublisher,
) -> Result<(), PhysicsSceneError> {
    let mut publisher = publisher;
    let mut scene = match RuntimeScene::create(arguments.dimension) {
        Ok(scene) => scene,
        Err(error) => {
            return publish_failure(publisher, error, unavailable_cleanup());
        }
    };

    let assets = match geometry::create_assets(arguments.dimension) {
        Ok(assets) => assets,
        Err(error) => {
            let cleanup = cleanup_scene(&mut scene, CleanupResult::NotAttempted);
            let error = with_cleanup(error, &cleanup);
            drop(scene);
            return publish_failure(publisher, error, cleanup);
        }
    };

    let mut renderer = match WgpuRenderer::create(arguments.policy) {
        Ok(renderer) => renderer,
        Err(error) => {
            let cleanup = cleanup_scene(&mut scene, CleanupResult::NotAttempted);
            let error = PhysicsSceneError::debug("create requested WGPU renderer", error);
            let error = with_cleanup(error, &cleanup);
            drop(scene);
            return publish_failure(publisher, error, cleanup);
        }
    };

    let execution = execute_scene(
        &mut scene,
        &assets,
        &mut renderer,
        &arguments,
        &mut publisher,
    );

    drop(renderer);
    let cleanup = cleanup_scene(&mut scene, CleanupResult::Succeeded);
    drop(scene);

    complete_run(publisher, execution, cleanup)
}

fn execute_scene(
    scene: &mut RuntimeScene,
    assets: &engine_assets::RenderAssets,
    renderer: &mut WgpuRenderer,
    arguments: &Arguments,
    publisher: &mut RenderOutputPublisher,
) -> Result<SimulationReport, PhysicsSceneError> {
    let report = simulation::run(scene, arguments.steps)?;
    rendering::render_timeline(&report, renderer, assets, arguments.target_size, publisher)?;

    Ok(report)
}

fn cleanup_scene(scene: &mut RuntimeScene, renderer: CleanupResult) -> CleanupOutcome {
    let dynamic = scene.runtime.physics_scene_remove_body(scene.dynamic.owner);
    let floor = scene.runtime.physics_scene_remove_body(scene.floor.owner);
    let remaining_bodies = scene.runtime.physics_scene_body_count();

    CleanupOutcome {
        shutdown: CleanupResult::NotAttempted,
        close: body_cleanup_result(dynamic, floor, remaining_bodies),
        renderer,
    }
}

fn body_cleanup_result(
    dynamic: Result<(), engine_runtime::RuntimeError>,
    floor: Result<(), engine_runtime::RuntimeError>,
    remaining_bodies: usize,
) -> CleanupResult {
    let mut diagnostics = Vec::new();
    append_removal_error(&mut diagnostics, "dynamic body", dynamic);
    append_removal_error(&mut diagnostics, "fixed floor", floor);

    if remaining_bodies != 0 {
        diagnostics.push(format!("{remaining_bodies} bodies remain in the scene"));
    }

    if diagnostics.is_empty() {
        CleanupResult::Succeeded
    } else {
        CleanupResult::Failed {
            diagnostic: diagnostics.join("; "),
        }
    }
}

fn append_removal_error(
    diagnostics: &mut Vec<String>,
    role: &str,
    result: Result<(), engine_runtime::RuntimeError>,
) {
    if let Err(error) = result {
        diagnostics.push(format!("remove {role}: {error:?}"));
    }
}

fn complete_run(
    publisher: RenderOutputPublisher,
    execution: Result<SimulationReport, PhysicsSceneError>,
    cleanup: CleanupOutcome,
) -> Result<(), PhysicsSceneError> {
    match execution {
        Err(error) => publish_failure(publisher, with_cleanup(error, &cleanup), cleanup),
        Ok(report) => complete_success(publisher, report, cleanup),
    }
}

fn complete_success(
    publisher: RenderOutputPublisher,
    report: SimulationReport,
    cleanup: CleanupOutcome,
) -> Result<(), PhysicsSceneError> {
    if let Some(error) = cleanup_error(&cleanup) {
        return publish_failure(publisher, error, cleanup);
    }

    let receipt = PhysicsSceneReceipt::from_simulation(&report);
    if let Err(error) = publisher.write_physics_scene_state(&receipt) {
        let error = PhysicsSceneError::display("publish physics-scene state", error);
        return publish_failure(publisher, error, cleanup);
    }

    publisher
        .finish(cleanup)
        .map_err(|error| PhysicsSceneError::display("write physics-scene receipt", error))
}

fn cleanup_error(cleanup: &CleanupOutcome) -> Option<PhysicsSceneError> {
    let CleanupResult::Failed { diagnostic } = &cleanup.close else {
        return None;
    };

    Some(PhysicsSceneError::new(format!(
        "native physics-scene cleanup failed: {diagnostic}"
    )))
}

fn with_cleanup(error: PhysicsSceneError, cleanup: &CleanupOutcome) -> PhysicsSceneError {
    match &cleanup.close {
        CleanupResult::Failed { diagnostic } => {
            PhysicsSceneError::new(format!("{error}; scene cleanup failed: {diagnostic}"))
        }
        _ => error,
    }
}

fn unavailable_cleanup() -> CleanupOutcome {
    CleanupOutcome {
        shutdown: CleanupResult::NotAttempted,
        close: CleanupResult::NotAttempted,
        renderer: CleanupResult::NotAttempted,
    }
}

fn publish_failure(
    publisher: RenderOutputPublisher,
    error: PhysicsSceneError,
    cleanup: CleanupOutcome,
) -> Result<(), PhysicsSceneError> {
    let diagnostic = error.to_string();
    match publisher.finish_failure(PublishError::Execution(diagnostic), cleanup) {
        Ok(()) => Err(error),
        Err(receipt_error) => Err(PhysicsSceneError::display(
            "write physics-scene failure receipt",
            receipt_error,
        )),
    }
}
