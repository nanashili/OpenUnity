use serde::Serialize;

use super::simulation::SimulationReport;

/// Compatibility and provenance limits for this native-only route.
#[derive(Serialize)]
struct CompatibilityRecord {
    managed_colliders: &'static str,
    unity_physics_parity: &'static str,
}

/// Supplemental state record beside the standard render receipt.
#[derive(Serialize)]
pub(super) struct PhysicsSceneReceipt<'a> {
    schema_version: u32,
    observation_status: &'static str,
    scene_source: &'static str,
    physics_profile: &'static str,
    compatibility: CompatibilityRecord,
    dimension: &'static str,
    fixed_timestep_seconds: f32,
    step_count: usize,
    body_count: usize,
    snapshots: &'a [super::snapshot::SceneSnapshot; 3],
    collision_evidence: &'a super::simulation::CollisionEvidence,
}

impl<'a> PhysicsSceneReceipt<'a> {
    pub(super) fn from_simulation(report: &'a SimulationReport) -> Self {
        Self {
            schema_version: 1,
            observation_status: "complete",
            scene_source: "independently-authored",
            physics_profile: "openunity-native-collision-scene-v1",
            compatibility: CompatibilityRecord {
                managed_colliders: "unsupported",
                unity_physics_parity: "unverified",
            },
            dimension: report.dimension.name(),
            fixed_timestep_seconds: super::simulation::FIXED_TIMESTEP_SECONDS,
            step_count: report.step_count,
            body_count: 2,
            snapshots: &report.snapshots,
            collision_evidence: &report.collision,
        }
    }
}
