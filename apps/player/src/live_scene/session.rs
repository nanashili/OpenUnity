use engine_assets::RenderAssets;
use engine_render_api::{RenderFrame, TargetSize};
use serde::Serialize;

use crate::physics_scene::arguments::Dimension;
use crate::physics_scene::execution::cleanup_scene;
use crate::physics_scene::geometry::create_assets;
use crate::physics_scene::rendering::build_live_frame;
use crate::physics_scene::scene::RuntimeScene;
use crate::physics_scene::snapshot::{DynamicBodyState, capture_dynamic_body_state};
use crate::render_output::CleanupResult;

use super::error::LiveSceneError;

/// Reusable native collision scene, geometry, and fixed-step state.
pub(super) struct LiveSceneSession {
    scene: RuntimeScene,
    assets: RenderAssets,
    simulation_steps: usize,
}

#[derive(Clone, Copy, Debug, Serialize)]
pub(super) struct BodyRetirementSummary {
    pub(super) succeeded: bool,
    pub(super) remaining_body_count: usize,
}

#[derive(Clone, Copy, Debug, Serialize)]
pub(super) struct LiveExecutionSummary {
    execution_profile: &'static str,
    unity_physics_parity: &'static str,
    dimension: &'static str,
    presented_frame_requests: u64,
    simulation_step_count: usize,
    final_dynamic_body: DynamicBodyState,
    body_retirement: BodyRetirementSummary,
}

impl LiveSceneSession {
    pub(super) fn create(dimension: Dimension) -> Result<Self, LiveSceneError> {
        let assets = create_assets(dimension)?;
        let scene = RuntimeScene::create(dimension)?;

        Ok(Self {
            scene,
            assets,
            simulation_steps: 0,
        })
    }

    pub(super) fn step_fixed(&mut self) -> Result<(), LiveSceneError> {
        let next_step = self.simulation_steps.checked_add(1).ok_or_else(|| {
            LiveSceneError::new("live physics simulation step count exceeded its supported range")
        })?;

        self.scene.step_fixed()?;
        self.simulation_steps = next_step;
        Ok(())
    }

    pub(super) fn render_frame(
        &self,
        target_size: TargetSize,
        frame_number: u64,
    ) -> Result<RenderFrame<'_>, LiveSceneError> {
        build_live_frame(
            &self.scene,
            &self.assets,
            target_size,
            self.simulation_steps,
            frame_number,
        )
        .map_err(LiveSceneError::from)
    }

    pub(super) fn final_body_state(&self) -> Result<DynamicBodyState, LiveSceneError> {
        capture_dynamic_body_state(&self.scene, self.simulation_steps).map_err(LiveSceneError::from)
    }

    pub(super) fn execution_summary(
        &self,
        presented_frame_requests: u64,
        final_dynamic_body: DynamicBodyState,
        body_retirement: BodyRetirementSummary,
    ) -> LiveExecutionSummary {
        LiveExecutionSummary {
            execution_profile: "openunity-native-collision-scene-v1",
            unity_physics_parity: "unverified",
            dimension: self.scene.dimension().name(),
            presented_frame_requests,
            simulation_step_count: self.simulation_steps,
            final_dynamic_body,
            body_retirement,
        }
    }

    pub(super) fn retire_bodies(&mut self) -> Result<BodyRetirementSummary, LiveSceneError> {
        let cleanup = cleanup_scene(&mut self.scene, CleanupResult::Succeeded);
        let remaining_body_count = self.scene.body_count();

        match cleanup.close {
            CleanupResult::Succeeded if remaining_body_count == 0 => Ok(BodyRetirementSummary {
                succeeded: true,
                remaining_body_count,
            }),
            CleanupResult::Succeeded => Err(LiveSceneError::new(format!(
                "live physics-scene body retirement left {remaining_body_count} bodies"
            ))),
            CleanupResult::Failed { diagnostic } => Err(LiveSceneError::new(format!(
                "retire live physics-scene bodies: {diagnostic}"
            ))),
            CleanupResult::NotAttempted => Err(LiveSceneError::new(
                "live physics-scene body retirement was not attempted",
            )),
        }
    }
}
