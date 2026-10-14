//! Atomic publication, abort, and destruction of staged scene ownership groups.

use super::{Runtime, RuntimeError};
use engine_object::ObjectId;
use std::thread;

impl Runtime {
    /// Atomically move the entire staged graph into this exact published runtime.
    /// # Errors
    /// Rejects invalid transactions or preparation failures before graph visibility.
    pub fn scene_publish(&mut self, handle: ObjectId) -> Result<(), RuntimeError> {
        let scene = self
            .scenes
            .handles
            .get(handle)
            .ok_or(RuntimeError::InvalidScene)?;
        if scene.thread != thread::current().id() {
            return Err(RuntimeError::StagingBusy);
        }
        if self.scenes.staging != Some(handle) || scene.staged.is_none() {
            return Err(RuntimeError::StagingBusy);
        }
        self.scene_lifecycle_ready(true)?;

        let staged_graph = scene.staged.as_ref().ok_or(RuntimeError::StagingBusy)?;
        let prepared_hierarchy = self
            .hierarchy
            .prepare_merge(handle, &staged_graph.hierarchy, &scene.roots)
            .map_err(crate::hierarchy::map_hierarchy_error)?;
        let mut scripts = self.lifecycle.scripts.clone();
        scripts.extend(
            staged_graph
                .lifecycle
                .scripts
                .iter()
                .map(|(id, script)| (*id, script.clone())),
        );
        let mut ownership = self.scenes.ownership.clone();
        ownership.extend(scene.ids.iter().map(|id| (*id, handle)));
        self.objects
            .prepare_append(&staged_graph.objects)
            .map_err(|_| RuntimeError::CountOverflow)?;
        self.transforms
            .try_reserve(staged_graph.transforms.len())
            .map_err(|_| RuntimeError::CountOverflow)?;

        self.hierarchy
            .commit_merge(prepared_hierarchy)
            .expect("prepared scene graph merge remains current during exclusive publication");
        let scene = self
            .scenes
            .handles
            .get_mut(handle)
            .expect("validated scene handle remains present during publication");
        let staged_graph = scene
            .staged
            .take()
            .expect("validated staged graph remains present during publication");
        scene.roots.clear();

        self.objects.append_reserved(staged_graph.objects);
        self.transforms.extend(staged_graph.transforms);
        self.lifecycle.scripts = scripts;
        self.scenes.ownership = ownership;
        self.scenes.staging = None;
        Ok(())
    }

    /// Consume unpublished storage without lifecycle events or identity reuse.
    /// # Errors
    /// Rejects foreign, stale, published handles, and owner thread violations.
    pub fn scene_abort(&mut self, handle: ObjectId) -> Result<(), RuntimeError> {
        let scene = self.scene(handle)?;
        if self.scenes.staging != Some(handle) || scene.staged.is_none() {
            return Err(RuntimeError::StagingBusy);
        }
        self.scenes.handles.remove(handle);
        self.scenes.staging = None;
        Ok(())
    }

    /// Detach and queue every imported owner as one hierarchy transition.
    /// # Errors
    /// Rejects invalid, staged, repeated, or faulted operations before mutation.
    pub fn scene_destroy(&mut self, handle: ObjectId) -> Result<(), RuntimeError> {
        let scene = self.scene(handle)?;
        if self.scene_staging() || scene.staged.is_some() || scene.destroy_requested {
            return Err(RuntimeError::StagingBusy);
        }
        self.scene_lifecycle_ready(true)?;

        let owners = scene.owners.clone();
        let transforms = self.live_scene_transforms(&owners)?;
        let prepared_hierarchy = self
            .hierarchy
            .prepare_scene_detachment(handle, &transforms)
            .map_err(crate::hierarchy::map_hierarchy_error)?;
        let prepared_requests = self.prepare_scene_destroy_requests(&owners)?;

        self.hierarchy
            .commit_change(prepared_hierarchy)
            .expect("prepared scene detachment remains current during exclusive destruction");
        self.commit_scene_destroy_requests(prepared_requests);
        let scene = self
            .scenes
            .handles
            .get_mut(handle)
            .expect("validated scene handle remains present during destruction");
        scene.destroy_requested = true;
        Ok(())
    }

    pub(super) fn live_scene_transforms(
        &self,
        owners: &[ObjectId],
    ) -> Result<Vec<ObjectId>, RuntimeError> {
        owners
            .iter()
            .copied()
            .filter(|owner| self.is_alive(*owner))
            .map(|owner| self.gameobject_transform(owner))
            .collect()
    }
}
