//! Scene ownership and readiness checks for runtime hierarchy mutations.

use super::Runtime;
use crate::RuntimeError;
use engine_object::ObjectId;
use std::thread;

impl Runtime {
    pub(crate) fn scene_hierarchy_owner(&self, id: ObjectId) -> Option<ObjectId> {
        self.scenes.ownership.get(&id.to_raw()).copied()
    }

    pub(crate) fn scene_hierarchy_owners_match(&self, first: ObjectId, second: ObjectId) -> bool {
        self.scene_hierarchy_owner(first) == self.scene_hierarchy_owner(second)
    }

    pub(crate) fn scene_hierarchy_admission(&self, id: ObjectId) -> Result<(), RuntimeError> {
        let Some(scene_handle) = self.scene_hierarchy_owner(id) else {
            return Ok(());
        };
        let scene = self
            .scenes
            .handles
            .get(scene_handle)
            .ok_or(RuntimeError::InvalidKind)?;

        if scene.destroy_requested {
            return Err(RuntimeError::InvalidKind);
        }
        if scene.thread != thread::current().id() || scene.staged.is_some() {
            return Err(RuntimeError::StagingBusy);
        }
        Ok(())
    }
}
