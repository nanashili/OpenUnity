//! Scene ownership release with prepared dynamic-body retirement.

use super::{Runtime, RuntimeError, Scene};
use engine_object::ObjectId;

/// Silently release a published scene after all owned resources are prepared.
///
/// The physics preparation/commit pair is intentionally kept around the object
/// ownership validation. A failed preparation leaves the scene handle and all
/// unrelated runtime resources available for retry.
impl Runtime {
    /// Release one complete scene ownership group, including hidden bodies.
    /// # Errors
    /// Rejects staged scenes, unfinished callback delivery, invalid ownership,
    /// or a failed dynamic-body retirement without mutating the scene handle.
    pub fn scene_release(&mut self, handle: ObjectId) -> Result<(), RuntimeError> {
        let scene = self.scene(handle)?;
        if self.scene_staging() || scene.staged.is_some() {
            return Err(RuntimeError::StagingBusy);
        }

        let object_ids = self.scene_release_object_ids(scene)?;
        let released_ids = object_ids.iter().map(|id| id.to_raw()).collect();
        self.scene_release_lifecycle_ready(&released_ids)?;

        // Detach the registry value before borrowing independent object and
        // physics fields. The detached entry restores itself on failure.
        let detached_scene = self
            .scenes
            .handles
            .detach(handle)
            .ok_or(RuntimeError::InvalidScene)?;

        // This composite helper prepares ObjectStore::prepare_release,
        // retires the matching committed/reserved backend body, and commits
        // both stores only after every fallible step succeeds.
        if let Err(error) = self.physics_release_objects(&object_ids) {
            detached_scene.restore(&mut self.scenes.handles);
            return Err(error);
        }

        detached_scene.discard();
        self.scene_release_lifecycle(&released_ids);
        self.transforms
            .retain(|id, _| !released_ids.contains(&id.to_raw()));
        self.scenes
            .ownership
            .retain(|id, _| !released_ids.contains(id));
        Ok(())
    }

    fn scene_release_object_ids(&self, scene: &Scene) -> Result<Vec<ObjectId>, RuntimeError> {
        let mut raw_ids = std::collections::BTreeSet::new();
        for raw in &scene.ids {
            ObjectId::from_raw(*raw).ok_or(RuntimeError::InvalidObject)?;
            raw_ids.insert(*raw);
        }

        for owner in scene.owners.iter().copied().filter(|id| self.is_alive(*id)) {
            let count = self.objects.owner_component_count(owner)?;
            for index in 0..count {
                let id = self.objects.owner_component_at(owner, index)?;
                raw_ids.insert(id.to_raw());
            }
        }

        for script in self.scene_owned_scripts(&scene.ids) {
            raw_ids.insert(script.to_raw());
        }

        // The attachment carries a reserved Rigidbody even before it is
        // published into the owner's component catalog.
        if let Some((owner_id, body_id)) = self.physics_attachment_ids()
            && (raw_ids.contains(&owner_id.to_raw()) || raw_ids.contains(&body_id.to_raw()))
        {
            raw_ids.insert(body_id.to_raw());
        }

        raw_ids
            .into_iter()
            .map(|raw| ObjectId::from_raw(raw).ok_or(RuntimeError::InvalidObject))
            .collect()
    }
}
