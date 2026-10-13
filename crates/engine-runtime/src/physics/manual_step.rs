//! Runtime composition for Script-mode all-body manual simulation.

use std::collections::HashMap;

use engine_identifiers::ObjectId;
use engine_math::Vector;
use engine_physics_api::{BodyId, SimulationMode};

use super::attachment_registry::MAX_PHYSICS_ATTACHMENTS;
use crate::{Runtime, RuntimeError, TransformData};

/// One exact attachment admitted before backend numeric preparation.
#[derive(Clone, Copy, Debug)]
pub(super) struct ManualStepAdmission {
    pub(super) body_id: ObjectId,
    pub(super) owner_id: ObjectId,
    pub(super) transform_id: ObjectId,
}

/// Bounded stack snapshot in ascending body identity order.
pub(super) struct ManualStepAdmissions {
    entries: [Option<ManualStepAdmission>; MAX_PHYSICS_ATTACHMENTS],
    len: usize,
}

impl ManualStepAdmissions {
    fn new() -> Self {
        Self {
            entries: [None; MAX_PHYSICS_ATTACHMENTS],
            len: 0,
        }
    }

    fn push(&mut self, admission: ManualStepAdmission) {
        self.entries[self.len] = Some(admission);
        self.len += 1;
    }

    fn len(&self) -> usize {
        self.len
    }

    fn iter(&self) -> impl Iterator<Item = ManualStepAdmission> + '_ {
        self.entries[..self.len].iter().flatten().copied()
    }
}

impl Runtime {
    /// Simulate the full committed set and publish every Transform position.
    pub fn physics_simulate(&mut self, duration: f32) -> Result<(), RuntimeError> {
        let admissions = self.manual_step_admission(duration)?;
        let gravity = self.physics_gravity();
        let plan = self
            .physics
            .world
            .prepare_step(duration, gravity)
            .map_err(RuntimeError::from)?;
        let positions = prepare_step_positions(&admissions, &plan)?;
        let transforms = PreparedTransformPositions::new(&mut self.transforms, positions)?;

        self.physics
            .world
            .commit_step(plan)
            .map_err(RuntimeError::from)?;
        transforms.publish();
        Ok(())
    }

    /// Validate every committed owner before backend profile and motion checks.
    pub(super) fn manual_step_admission(
        &self,
        duration: f32,
    ) -> Result<ManualStepAdmissions, RuntimeError> {
        self.lifecycle_ready()?;
        if self.scene_staging() {
            return Err(RuntimeError::StagingBusy);
        }
        if self.physics.mode != SimulationMode::Script {
            return Err(RuntimeError::PhysicsUnsupported);
        }
        validate_duration(duration)?;

        let mut admissions = ManualStepAdmissions::new();
        for attachment in self.physics.ordered_attachments() {
            if !attachment.committed {
                continue;
            }

            self.validate_physics_step_owner(attachment.owner_id, attachment.body_id)?;
            self.validate_supported_owner_transform(attachment.owner_id, attachment.transform_id)?;
            if attachment.is_dirty() {
                return Err(RuntimeError::Lifecycle);
            }
            self.physics
                .world
                .body(BodyId::from(attachment.body_id))
                .map_err(RuntimeError::from)?;

            admissions.push(ManualStepAdmission {
                body_id: attachment.body_id,
                owner_id: attachment.owner_id,
                transform_id: attachment.transform_id,
            });
        }

        Ok(admissions)
    }
}

/// Pair backend candidates with attachments before acquiring Transform storage.
fn prepare_step_positions(
    admissions: &ManualStepAdmissions,
    plan: &engine_physics_3d::PreparedStep,
) -> Result<Vec<StepTransformPosition>, RuntimeError> {
    let mut positions = Vec::new();
    positions
        .try_reserve_exact(admissions.len())
        .map_err(|_| RuntimeError::CountOverflow)?;

    let mut candidates = plan.bodies();
    for admission in admissions.iter() {
        let Some((body_id, state)) = candidates.next() else {
            return Err(RuntimeError::InvalidObject);
        };
        if ObjectId::from(body_id) != admission.body_id {
            return Err(RuntimeError::InvalidObject);
        }

        positions.push(StepTransformPosition {
            transform_id: admission.transform_id,
            owner_id: admission.owner_id,
            position: state.position(),
        });
    }

    if candidates.next().is_some() {
        return Err(RuntimeError::InvalidObject);
    }

    Ok(positions)
}

#[derive(Clone, Copy, Debug)]
struct StepTransformPosition {
    transform_id: ObjectId,
    owner_id: ObjectId,
    position: Vector,
}

/// Hold the exact Transform map borrow from preflight through publication.
struct PreparedTransformPositions<'runtime> {
    transforms: &'runtime mut HashMap<ObjectId, TransformData>,
    positions: Vec<StepTransformPosition>,
}

impl<'runtime> PreparedTransformPositions<'runtime> {
    fn new(
        transforms: &'runtime mut HashMap<ObjectId, TransformData>,
        positions: Vec<StepTransformPosition>,
    ) -> Result<Self, RuntimeError> {
        validate_transform_targets(transforms, &positions)?;

        Ok(Self {
            transforms,
            positions,
        })
    }

    /// Apply preflighted value writes with no fallible operation after commit.
    fn publish(self) {
        for prepared in self.positions {
            let target = self
                .transforms
                .get_mut(&prepared.transform_id)
                .expect("preflighted Transform remains in exclusively borrowed storage");
            debug_assert_eq!(target.owner, prepared.owner_id);
            target.position = prepared.position;
        }
    }
}

fn validate_transform_targets(
    transforms: &HashMap<ObjectId, TransformData>,
    positions: &[StepTransformPosition],
) -> Result<(), RuntimeError> {
    for (index, prepared) in positions.iter().enumerate() {
        let target = transforms
            .get(&prepared.transform_id)
            .ok_or(RuntimeError::InvalidObject)?;
        if target.owner != prepared.owner_id {
            return Err(RuntimeError::InvalidObject);
        }

        if positions[..index]
            .iter()
            .any(|prior| prior.transform_id == prepared.transform_id)
        {
            return Err(RuntimeError::InvalidObject);
        }
    }

    Ok(())
}

fn validate_duration(duration: f32) -> Result<(), RuntimeError> {
    if !duration.is_finite() || duration <= 0.0 {
        return Err(RuntimeError::Numeric);
    }
    Ok(())
}
