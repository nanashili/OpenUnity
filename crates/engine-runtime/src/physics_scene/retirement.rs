//! Candidate collision-state retirement for object destruction and release.

use engine_identifiers::ObjectId;

use super::CollisionSceneState;
use crate::RuntimeError;

impl CollisionSceneState {
    pub(crate) fn prepare_retirement(&self, ids: &[ObjectId]) -> Result<Self, RuntimeError> {
        let mut candidate = self.clone();

        for owner in ids {
            if candidate.contains_owner(*owner) {
                candidate.remove_body(*owner)?;
            }
        }

        Ok(candidate)
    }
}
