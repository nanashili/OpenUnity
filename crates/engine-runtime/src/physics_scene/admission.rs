//! Shared lifecycle admission for collision-scene mutations.

use crate::{Runtime, RuntimeError};

impl Runtime {
    pub(super) fn collision_scene_admission_ready(&self) -> Result<(), RuntimeError> {
        self.lifecycle_ready()?;
        if self.scene_staging() {
            return Err(RuntimeError::StagingBusy);
        }
        Ok(())
    }
}
