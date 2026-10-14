//! Validation and construction of an exclusively owned, unpublished scene graph.

use super::{Runtime, RuntimeError, Scene};
use engine_object::ObjectId;
use engine_scene::HierarchyTarget;
use engine_serialization::ScenePlan;
use std::collections::BTreeMap;
use std::thread;

struct UnpublishedGraph {
    graph: Box<Runtime>,
    local: BTreeMap<i64, ObjectId>,
    owners: Vec<ObjectId>,
}

impl Runtime {
    /// Validate and allocate a completely unpublished graph with consumed identities.
    /// # Errors
    /// Rejects invalid plans, active transactions, lifecycle boundaries and exhausted IDs.
    pub fn scene_stage(&mut self, plan: &ScenePlan) -> Result<ObjectId, RuntimeError> {
        if self.scene_staging() {
            return Err(RuntimeError::StagingBusy);
        }
        self.scene_lifecycle_ready(true)?;
        validate_plan(plan)?;
        let scene = UnpublishedGraph::construct(plan)?.into_scene(plan);
        let handle = self
            .scenes
            .handles
            .insert(scene)
            .map_err(|_| RuntimeError::IdentityExhausted)?;
        self.scenes.staging = Some(handle);
        Ok(handle)
    }
}

fn validate_plan(plan: &ScenePlan) -> Result<(), RuntimeError> {
    plan.validate()
        .map_err(|_| RuntimeError::InvalidScenePlan)?;
    for owner in &plan.game_objects {
        if owner.components.first().copied()
            != plan
                .transforms
                .iter()
                .find(|transform| transform.owner == owner.file_id)
                .map(|transform| transform.file_id)
        {
            return Err(RuntimeError::InvalidScenePlan);
        }
    }
    Ok(())
}

impl UnpublishedGraph {
    fn construct(plan: &ScenePlan) -> Result<Self, RuntimeError> {
        let mut staged = Self {
            graph: Box::<Runtime>::default(),
            local: BTreeMap::new(),
            owners: Vec::with_capacity(plan.game_objects.len()),
        };
        staged.allocate_owners(plan)?;
        staged.reserve_scripts(plan)?;
        staged.commit_scripts(plan)?;
        staged.configure_scripts(plan)?;
        staged.configure_transforms(plan)?;
        staged.validate_composed_transforms(plan)?;
        Ok(staged)
    }

    fn allocate_owners(&mut self, plan: &ScenePlan) -> Result<(), RuntimeError> {
        for owner in &plan.game_objects {
            let id = self
                .graph
                .create_gameobject(&owner.name)
                .map_err(|_| RuntimeError::IdentityExhausted)?;
            self.owners.push(id);
            self.local.insert(owner.file_id, id);
            let transform = plan
                .transforms
                .iter()
                .find(|transform| transform.owner == owner.file_id)
                .ok_or(RuntimeError::InvalidScenePlan)?;
            self.local
                .insert(transform.file_id, self.graph.gameobject_transform(id)?);
            self.graph.gameobject_set_active(id, owner.active)?;
        }
        Ok(())
    }

    fn reserve_scripts(&mut self, plan: &ScenePlan) -> Result<(), RuntimeError> {
        for script in &plan.scripts {
            let owner = *self
                .local
                .get(&script.owner)
                .ok_or(RuntimeError::InvalidScenePlan)?;
            let id = self
                .graph
                .component_reserve(owner, script.execution_order)?;
            self.local.insert(script.file_id, id);
        }
        Ok(())
    }

    fn commit_scripts(&mut self, plan: &ScenePlan) -> Result<(), RuntimeError> {
        // Commit only inside the isolated graph, in authored component order.
        for owner in &plan.game_objects {
            for file in owner.components.iter().skip(1) {
                let id = *self.local.get(file).ok_or(RuntimeError::InvalidScenePlan)?;
                self.graph.component_commit(id)?;
            }
        }
        Ok(())
    }

    fn configure_scripts(&mut self, plan: &ScenePlan) -> Result<(), RuntimeError> {
        for script in &plan.scripts {
            self.graph
                .behaviour_set_enabled(self.local[&script.file_id], script.enabled)?;
        }
        Ok(())
    }

    fn configure_transforms(&mut self, plan: &ScenePlan) -> Result<(), RuntimeError> {
        for transform in &plan.transforms {
            let id = self.local[&transform.file_id];
            self.graph
                .transform_write(id, 1, &transform.local_position)
                .map_err(|_| RuntimeError::InvalidScenePlan)?;
            self.graph
                .transform_write(id, 2, &transform.local_rotation)
                .map_err(|_| RuntimeError::InvalidScenePlan)?;
            self.graph
                .transform_write(id, 3, &transform.local_scale)
                .map_err(|_| RuntimeError::InvalidScenePlan)?;
        }

        for parent in &plan.transforms {
            let parent_id = self.local[&parent.file_id];
            for child_file_id in &parent.children {
                let child_id = self.local[child_file_id];
                self.graph
                    .hierarchy
                    .reparent(child_id, HierarchyTarget::Child(parent_id))
                    .map_err(crate::hierarchy::map_hierarchy_error)?;
            }
        }
        Ok(())
    }

    fn validate_composed_transforms(&self, plan: &ScenePlan) -> Result<(), RuntimeError> {
        // Reject composed numerical failures before allowing managed construction.
        for transform in &plan.transforms {
            let id = self.local[&transform.file_id];
            self.graph
                .transform_read(id, 4)
                .map_err(|_| RuntimeError::InvalidScenePlan)?;
            self.graph
                .transform_read(id, 5)
                .map_err(|_| RuntimeError::InvalidScenePlan)?;
        }
        Ok(())
    }

    fn into_scene(self, plan: &ScenePlan) -> Scene {
        let roots = plan.roots.iter().map(|id| self.local[id]).collect();
        let ids = self.local.values().map(|id| id.to_raw()).collect();
        Scene {
            thread: thread::current().id(),
            local: self.local,
            ids,
            owners: self.owners,
            roots,
            staged: Some(self.graph),
            destroy_requested: false,
        }
    }
}
