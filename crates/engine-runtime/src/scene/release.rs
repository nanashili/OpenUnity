//! Complete ownership validation and silent scene resource teardown.

use super::{Runtime, RuntimeError};
use engine_object::ObjectId;

impl Runtime {
    /// Silently release published residual scene resources, including a faulted session.
    /// # Errors
    /// Rejects invalid/staged handles, an active transaction, or any in-flight callback.
    pub fn scene_release(&mut self, handle: ObjectId) -> Result<(), RuntimeError> {
        let scene = self.scene(handle)?;
        if self.scene_staging() || scene.staged.is_some() {
            return Err(RuntimeError::StagingBusy);
        }
        self.scene_lifecycle_ready(false)?;
        let mut ids = scene.ids.clone();
        // Include committed and reserved scripts attached after loading to imported owners.
        ids.extend(
            self.scene_owned_scripts(&ids)
                .into_iter()
                .map(ObjectId::to_raw),
        );
        let object_ids: Vec<_> = ids
            .iter()
            .filter_map(|raw| ObjectId::from_raw(*raw))
            .collect();
        self.objects.validate_release(&object_ids)?;

        self.scenes
            .handles
            .remove(handle)
            .ok_or(RuntimeError::InvalidScene)?;
        self.scene_release_lifecycle(&ids);
        // Lifecycle cleanup does not mutate object metadata, so the preflight
        // ownership validation remains valid through this release phase.
        self.objects
            .release_objects(&object_ids)
            .expect("validated scene ownership release");
        self.transforms.retain(|id, _| !ids.contains(&id.to_raw()));
        self.scenes.ownership.retain(|id, _| !ids.contains(id));
        Ok(())
    }
}
