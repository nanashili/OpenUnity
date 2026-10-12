//! Explicit Physics.SyncTransforms composition.
//!
//! These helpers leave live body state in `MotionWorld`; callers prepare a
//! candidate, publish it there, then acknowledge revisions infallibly.

use engine_math::{Vector, finite};
use engine_physics_api::{BodyState, PhysicsError, SimulationMode};

use super::body_properties::BodyProperties;

/// Empty worlds still accept SyncTransforms in both admitted modes.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum SyncOutcome {
    Empty,
    Clean,
    Published,
}

/// Synchronize an authored Transform position into a committed backend body.
///
/// The caller validates owner/activity/pending state and invokes
/// `MotionWorld::set_body` with the returned candidate before acknowledging
/// the revision. No lookup, allocation, callback, or clock mutation occurs.
pub(crate) fn prepare_sync_transforms(
    mode: SimulationMode,
    body: Option<(BodyState, &BodyProperties)>,
    transform_position: Vector,
) -> Result<(SyncOutcome, Option<BodyState>), PhysicsError> {
    finite(&transform_position)?;
    let Some((body, metadata)) = body else {
        return Ok((SyncOutcome::Empty, None));
    };
    match mode {
        SimulationMode::Fixed | SimulationMode::Script => {}
    }
    let candidate = metadata.sync_authored_position(body, transform_position)?;
    match candidate {
        Some((body, _revision)) => Ok((SyncOutcome::Published, Some(body))),
        None => Ok((SyncOutcome::Clean, None)),
    }
}
