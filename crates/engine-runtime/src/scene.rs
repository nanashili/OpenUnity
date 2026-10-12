//! Independently designed unpublished scene transactions; no reference loader code.
use super::{Runtime, RuntimeError};
use engine_object::{ObjectId, Registry};
use std::collections::{BTreeMap, BTreeSet};
use std::thread::{self, ThreadId};

mod release;
mod staging;

#[derive(Debug)]
struct Scene {
    thread: ThreadId,
    local: BTreeMap<i64, ObjectId>,
    ids: BTreeSet<i64>,
    owners: Vec<ObjectId>,
    roots: Vec<ObjectId>,
    staged: Option<Box<Runtime>>,
    destroy_requested: bool,
}

#[derive(Debug, Default)]
pub(super) struct Scenes {
    handles: Registry<Scene>,
    staging: Option<ObjectId>,
    ownership: BTreeMap<i64, ObjectId>,
}

impl Runtime {
    /// Whether all ordinary engine calls must reject access during restoration.
    pub fn scene_staging(&self) -> bool {
        self.scenes.staging.is_some()
    }

    fn scene(&self, handle: ObjectId) -> Result<&Scene, RuntimeError> {
        let scene = self
            .scenes
            .handles
            .get(handle)
            .ok_or(RuntimeError::InvalidScene)?;
        if scene.thread != thread::current().id() {
            return Err(RuntimeError::StagingBusy);
        }
        Ok(scene)
    }

    /// Resolve one validated local file identity through the trusted loader route.
    /// # Errors
    /// Rejects stale/foreign handles, owner thread violations, and unknown/dead identities.
    pub fn scene_object(&self, handle: ObjectId, file: i64) -> Result<ObjectId, RuntimeError> {
        let scene = self.scene(handle)?;
        if self.scenes.staging.is_some_and(|active| active != handle) {
            return Err(RuntimeError::StagingBusy);
        }
        let id = *scene.local.get(&file).ok_or(RuntimeError::InvalidObject)?;
        if scene
            .staged
            .as_ref()
            .map_or_else(|| self.is_alive(id), |graph| graph.is_alive(id))
        {
            Ok(id)
        } else {
            Err(RuntimeError::InvalidObject)
        }
    }

    /// Atomically move the entire staged graph into this exact published runtime.
    /// # Errors
    /// Rejects invalid/terminal transactions or capacity preparation failure before visibility.
    pub fn scene_publish(&mut self, handle: ObjectId) -> Result<(), RuntimeError> {
        let scene = self.scene(handle)?;
        if self.scenes.staging != Some(handle) || scene.staged.is_none() {
            return Err(RuntimeError::StagingBusy);
        }
        self.scene_lifecycle_ready(true)?;
        let graph = scene.staged.as_ref().ok_or(RuntimeError::StagingBusy)?;
        let mut scripts = self.lifecycle.scripts.clone();
        scripts.extend(
            graph
                .lifecycle
                .scripts
                .iter()
                .map(|(id, script)| (*id, script.clone())),
        );
        let mut ownership = self.scenes.ownership.clone();
        ownership.extend(scene.ids.iter().map(|id| (*id, handle)));
        let graph = self
            .scenes
            .handles
            .get(handle)
            .and_then(|scene| scene.staged.as_ref())
            .ok_or(RuntimeError::StagingBusy)?;
        self.objects
            .prepare_append(&graph.objects)
            .map_err(|_| RuntimeError::CountOverflow)?;
        self.transforms
            .try_reserve(graph.transforms.len())
            .map_err(|_| RuntimeError::CountOverflow)?;
        // Everything following this point consists only of owned moves into reserved storage.
        let graph = self
            .scenes
            .handles
            .get_mut(handle)
            .and_then(|scene| scene.staged.take())
            .ok_or(RuntimeError::StagingBusy)?;
        self.objects.append_reserved(graph.objects);
        self.transforms.extend(graph.transforms);
        self.lifecycle.scripts = scripts;
        self.scenes.ownership = ownership;
        self.scenes.staging = None;
        Ok(())
    }

    /// Consume unpublished storage without lifecycle events or identity reuse.
    /// # Errors
    /// Rejects foreign/stale/published handles and owner thread violations.
    pub fn scene_abort(&mut self, handle: ObjectId) -> Result<(), RuntimeError> {
        let scene = self.scene(handle)?;
        if self.scenes.staging != Some(handle) || scene.staged.is_none() {
            return Err(RuntimeError::StagingBusy);
        }
        self.scenes.handles.remove(handle);
        self.scenes.staging = None;
        Ok(())
    }

    /// Detach and queue every imported owner, regardless of its current hierarchy.
    /// # Errors
    /// Rejects invalid, staged, repeated or faulted operations before mutation.
    pub fn scene_destroy(&mut self, handle: ObjectId) -> Result<(), RuntimeError> {
        let scene = self.scene(handle)?;
        if self.scene_staging() || scene.staged.is_some() || scene.destroy_requested {
            return Err(RuntimeError::StagingBusy);
        }
        self.scene_lifecycle_ready(true)?;
        let owners = scene.owners.clone();
        for owner in owners
            .iter()
            .copied()
            .filter(|id| self.is_alive(*id))
            .collect::<Vec<_>>()
        {
            let transform = self.gameobject_transform(owner)?;
            self.transform_set_parent(transform, None, false)?;
        }
        for owner in owners {
            if self.is_alive(owner) {
                self.object_destroy_deferred(owner)?;
            }
        }
        self.scenes
            .handles
            .get_mut(handle)
            .ok_or(RuntimeError::InvalidScene)?
            .destroy_requested = true;
        Ok(())
    }

    pub(super) fn scene_check_parent(
        &self,
        id: ObjectId,
        parent: ObjectId,
    ) -> Result<(), RuntimeError> {
        if self.scenes.ownership.get(&id.to_raw()) != self.scenes.ownership.get(&parent.to_raw()) {
            return Err(RuntimeError::Unsupported);
        }
        Ok(())
    }

    pub(super) fn scene_update_roots(
        &mut self,
        id: ObjectId,
        old: Option<ObjectId>,
        new: Option<ObjectId>,
    ) {
        let Some(&handle) = self.scenes.ownership.get(&id.to_raw()) else {
            return;
        };
        let Some(scene) = self.scenes.handles.get_mut(handle) else {
            return;
        };
        if old.is_none() {
            scene.roots.retain(|root| *root != id);
        }
        if new.is_none() {
            scene.roots.push(id);
        }
    }

    /// Return actual child order, authored scene root order, or zero for unowned roots.
    /// # Errors
    /// Rejects invalid/wrong-kind identities or an unrepresentable index.
    pub fn transform_sibling_index(&self, id: ObjectId) -> Result<i32, RuntimeError> {
        let data = self.transform(id)?;
        let index = if let Some(parent) = data.parent {
            self.transform(parent)?
                .children
                .iter()
                .position(|child| *child == id)
                .ok_or(RuntimeError::InvalidKind)?
        } else if let Some(&handle) = self.scenes.ownership.get(&id.to_raw()) {
            self.scenes
                .handles
                .get(handle)
                .ok_or(RuntimeError::InvalidScene)?
                .roots
                .iter()
                .filter(|root| self.is_alive(**root))
                .position(|root| *root == id)
                .ok_or(RuntimeError::InvalidKind)?
        } else {
            0
        };
        i32::try_from(index).map_err(|_| RuntimeError::CountOverflow)
    }
}
