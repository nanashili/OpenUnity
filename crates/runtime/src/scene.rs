//! Independently designed unpublished scene transactions; no reference loader code.
use super::{Runtime, RuntimeError};
use openunity_object_model::{ObjectId, Registry};
use openunity_scene::ScenePlan;
use std::collections::{BTreeMap, BTreeSet};
use std::thread::{self, ThreadId};

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

    /// Validate and allocate a completely unpublished graph with consumed identities.
    /// # Errors
    /// Rejects invalid plans, active transactions, lifecycle boundaries and exhausted IDs.
    pub fn scene_stage(&mut self, plan: &ScenePlan) -> Result<ObjectId, RuntimeError> {
        if self.scene_staging() {
            return Err(RuntimeError::StagingBusy);
        }
        self.scene_lifecycle_ready(true)?;
        plan.validate()
            .map_err(|_| RuntimeError::InvalidScenePlan)?;
        for owner in &plan.game_objects {
            if owner.components.first().copied()
                != plan
                    .transforms
                    .iter()
                    .find(|t| t.owner == owner.file_id)
                    .map(|t| t.file_id)
            {
                return Err(RuntimeError::InvalidScenePlan);
            }
        }
        let mut graph = Box::<Runtime>::default();
        let mut local = BTreeMap::new();
        let mut owners = Vec::with_capacity(plan.game_objects.len());
        for owner in &plan.game_objects {
            let id = graph
                .create_gameobject(&owner.name)
                .map_err(|_| RuntimeError::IdentityExhausted)?;
            owners.push(id);
            local.insert(owner.file_id, id);
            let transform = plan
                .transforms
                .iter()
                .find(|t| t.owner == owner.file_id)
                .ok_or(RuntimeError::InvalidScenePlan)?;
            local.insert(transform.file_id, graph.gameobject_transform(id)?);
            graph.gameobject_set_active(id, owner.active)?;
        }
        for script in &plan.scripts {
            let owner = *local
                .get(&script.owner)
                .ok_or(RuntimeError::InvalidScenePlan)?;
            let id = graph.component_reserve(owner, script.execution_order)?;
            local.insert(script.file_id, id);
        }
        // Commit only inside the isolated graph, in authored component order.
        for owner in &plan.game_objects {
            for file in owner.components.iter().skip(1) {
                graph.component_commit(*local.get(file).ok_or(RuntimeError::InvalidScenePlan)?)?;
            }
        }
        for script in &plan.scripts {
            graph.behaviour_set_enabled(local[&script.file_id], script.enabled)?;
        }
        for transform in &plan.transforms {
            let id = local[&transform.file_id];
            graph
                .transform_write(id, 1, &transform.local_position)
                .map_err(|_| RuntimeError::InvalidScenePlan)?;
            graph
                .transform_write(id, 2, &transform.local_rotation)
                .map_err(|_| RuntimeError::InvalidScenePlan)?;
            graph
                .transform_write(id, 3, &transform.local_scale)
                .map_err(|_| RuntimeError::InvalidScenePlan)?;
            // All graph links were validated by ScenePlan; install ordered children directly.
            let data = graph.transform_mut(id)?;
            data.parent = (transform.parent != 0).then(|| local[&transform.parent]);
            data.children = transform.children.iter().map(|id| local[id]).collect();
        }
        // Validate composed numerical results before allowing managed construction.
        for transform in &plan.transforms {
            let id = local[&transform.file_id];
            graph
                .transform_read(id, 4)
                .map_err(|_| RuntimeError::InvalidScenePlan)?;
            graph
                .transform_read(id, 5)
                .map_err(|_| RuntimeError::InvalidScenePlan)?;
        }
        let roots = plan.roots.iter().map(|id| local[id]).collect();
        let ids = local.values().map(|id| id.to_raw()).collect();
        let handle = self
            .scenes
            .handles
            .insert(Scene {
                thread: thread::current().id(),
                local,
                ids,
                owners,
                roots,
                staged: Some(graph),
                destroy_requested: false,
            })
            .map_err(|_| RuntimeError::IdentityExhausted)?;
        self.scenes.staging = Some(handle);
        Ok(handle)
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
        // Everything following this point consists only of owned moves into reserved storage.
        let graph = self
            .scenes
            .handles
            .get_mut(handle)
            .and_then(|scene| scene.staged.take())
            .ok_or(RuntimeError::StagingBusy)?;
        self.objects.append_reserved(graph.objects);
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

    /// Silently release published residual scene resources, including a faulted session.
    /// # Errors
    /// Rejects invalid/staged handles, an active transaction, or any in-flight callback.
    pub fn scene_release(&mut self, handle: ObjectId) -> Result<(), RuntimeError> {
        let scene = self.scene(handle)?;
        if self.scene_staging() || scene.staged.is_some() {
            return Err(RuntimeError::StagingBusy);
        }
        self.scene_lifecycle_ready(false)?;
        let scene = self
            .scenes
            .handles
            .remove(handle)
            .ok_or(RuntimeError::InvalidScene)?;
        let mut ids = scene.ids;
        // Include committed and reserved scripts attached after loading to imported owners.
        ids.extend(
            self.scene_owned_scripts(&ids)
                .into_iter()
                .map(ObjectId::to_raw),
        );
        self.scene_release_lifecycle(&ids);
        for raw in &ids {
            if let Some(id) = ObjectId::from_raw(*raw) {
                self.objects.remove(id);
            }
        }
        self.scenes.ownership.retain(|id, _| !ids.contains(id));
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
