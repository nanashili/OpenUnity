//! Independently designed unpublished scene transactions; no reference loader code.
use super::{Runtime, RuntimeError};
use engine_object::{ObjectId, Registry};
use std::collections::{BTreeMap, BTreeSet};
use std::thread::{self, ThreadId};

mod hierarchy_admission;
mod release;
mod roots;
mod staging;
mod transactions;
mod validity;

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

    pub(crate) fn scene_snapshot_scene_ready(&self, handle: ObjectId) -> Result<(), RuntimeError> {
        let scene = self.scene(handle)?;
        if scene.staged.is_some() || scene.destroy_requested || self.scene_staging() {
            return Err(RuntimeError::StagingBusy);
        }
        Ok(())
    }

    pub(crate) fn scene_snapshot_target(
        &self,
        handle: ObjectId,
        local_file_id: i64,
    ) -> Result<(Option<ObjectId>, bool), RuntimeError> {
        let scene = self.scene(handle)?;
        let id = *scene
            .local
            .get(&local_file_id)
            .ok_or(RuntimeError::InvalidObject)?;
        if scene.owners.contains(&id) {
            return Ok((self.is_alive(id).then_some(id), !self.is_alive(id)));
        }
        if !self.is_alive(id) {
            return Err(RuntimeError::InvalidKind);
        }
        Err(RuntimeError::InvalidKind)
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

    /// Return actual child order, authored scene root order, or zero for unowned roots.
    /// # Errors
    /// Rejects invalid/wrong-kind identities or an unrepresentable index.
    pub fn transform_sibling_index(&self, id: ObjectId) -> Result<i32, RuntimeError> {
        self.transform(id)?;
        let index = self
            .hierarchy
            .sibling_index(id)
            .map_err(crate::hierarchy::map_hierarchy_error)?;
        i32::try_from(index).map_err(|_| RuntimeError::CountOverflow)
    }
}

#[cfg(test)]
#[path = "scene/snapshot_admission_tests.rs"]
mod snapshot_admission_tests;
