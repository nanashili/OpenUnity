//! Prepare and publish one atomic dirty-Transform batch.

use engine_identifiers::ObjectId;
use engine_physics_api::{BodyId, BodyState, PhysicsError, SimulationMode};

use super::body_properties::BodyProperties;
use crate::{Runtime, RuntimeError};

/// Owned backend states and exact authored revisions prepared before publication.
pub(super) struct PreparedSyncBatch {
    pub(super) body_states: Vec<(BodyId, BodyState)>,
    acknowledgements: Vec<(ObjectId, u64)>,
}

impl Runtime {
    /// Validate every owner before preparing any numeric body candidate.
    pub(super) fn prepare_physics_sync_batch(&self) -> Result<PreparedSyncBatch, RuntimeError> {
        self.validate_physics_sync_access()?;
        self.validate_sync_attachments()?;

        let dirty_count = self
            .physics
            .ordered_attachments()
            .filter(|attachment| attachment.committed && attachment.is_dirty())
            .count();
        let mut batch = PreparedSyncBatch::try_with_capacity(dirty_count)?;

        self.prepare_sync_candidates(&mut batch)?;
        Ok(batch)
    }

    /// Record the exact authored revisions only after the world batch commits.
    pub(super) fn commit_physics_sync_acknowledgements(&mut self, batch: PreparedSyncBatch) {
        for (body_id, prepared_revision) in batch.acknowledgements {
            let attachment = self
                .physics
                .attachment_by_body_mut(body_id)
                .expect("preflighted body keeps its exact attachment");
            attachment
                .properties
                .acknowledge_prepared_revision(prepared_revision);
        }
    }

    fn validate_sync_attachments(&self) -> Result<(), RuntimeError> {
        for attachment in self.physics.ordered_attachments() {
            if !attachment.committed {
                continue;
            }

            self.validate_physics_step_owner(attachment.owner_id, attachment.body_id)?;
            self.validate_supported_owner_transform(attachment.owner_id, attachment.transform_id)?;
            self.physics
                .world
                .body(BodyId::from(attachment.body_id))
                .map_err(RuntimeError::from)?;
        }

        Ok(())
    }

    fn prepare_sync_candidates(&self, batch: &mut PreparedSyncBatch) -> Result<(), RuntimeError> {
        for attachment in self.physics.ordered_attachments() {
            if !attachment.committed {
                continue;
            }

            let state = self
                .physics
                .world
                .body(BodyId::from(attachment.body_id))
                .map_err(RuntimeError::from)?;
            let position = self.transform(attachment.transform_id)?.position;
            let candidate =
                prepare_sync_body(self.physics.mode, state, &attachment.properties, position)
                    .map_err(RuntimeError::from)?;

            if let Some((state, revision)) = candidate {
                batch
                    .body_states
                    .push((BodyId::from(attachment.body_id), state));
                batch.acknowledgements.push((attachment.body_id, revision));
            }
        }

        Ok(())
    }
}

impl PreparedSyncBatch {
    fn try_with_capacity(capacity: usize) -> Result<Self, RuntimeError> {
        let mut body_states = Vec::new();
        body_states
            .try_reserve_exact(capacity)
            .map_err(|_| RuntimeError::CountOverflow)?;

        let mut acknowledgements = Vec::new();
        acknowledgements
            .try_reserve_exact(capacity)
            .map_err(|_| RuntimeError::CountOverflow)?;

        Ok(Self {
            body_states,
            acknowledgements,
        })
    }
}

/// Prepare one copied state without changing backend state or revision metadata.
fn prepare_sync_body(
    mode: SimulationMode,
    state: BodyState,
    properties: &BodyProperties,
    position: [f32; 3],
) -> Result<Option<(BodyState, u64)>, PhysicsError> {
    match mode {
        SimulationMode::Fixed | SimulationMode::Script => {}
    }

    properties.sync_authored_position(state, position)
}
