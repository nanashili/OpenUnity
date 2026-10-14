//! Published scene handle validity queries.

use super::Runtime;
use crate::RuntimeError;
use engine_object::ObjectId;
use std::thread;

impl Runtime {
    /// Whether an identity currently names a published, usable scene.
    ///
    /// A stale, staged, or destroy-requested handle is invalid. The owner
    /// thread is still required when the handle resolves to a live scene.
    /// # Errors
    /// Returns `StagingBusy` when the live scene is accessed from another thread.
    pub fn scene_is_valid(&self, handle: ObjectId) -> Result<bool, RuntimeError> {
        let Some(scene) = self.scenes.handles.get(handle) else {
            return Ok(false);
        };

        if scene.staged.is_some() || scene.destroy_requested {
            return Ok(false);
        }

        if scene.thread != thread::current().id() {
            return Err(RuntimeError::StagingBusy);
        }

        if self.scene_staging() {
            return Err(RuntimeError::StagingBusy);
        }

        Ok(true)
    }
}
