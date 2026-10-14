use super::unique::collect_unique;
use super::{HierarchyError, PreparedHierarchyChange, RootOwner, SceneHierarchy};
use engine_identifiers::ObjectId;
use std::collections::HashSet;

impl SceneHierarchy {
    /// Prepare removal of a unique, complete set of graph subtrees.
    pub fn prepare_removal(
        &self,
        transforms: &[ObjectId],
    ) -> Result<PreparedHierarchyChange, HierarchyError> {
        self.validate_all()?;
        let selected = collect_unique(transforms)?;
        let mut candidate = self.try_clone_graph()?;
        candidate.remove_selected_nodes(&selected)?;
        candidate.validate_all()?;
        Ok(PreparedHierarchyChange {
            base_revision: self.revision,
            candidate,
        })
    }

    /// Prepare removal of every Transform belonging to a released scene.
    pub fn prepare_scene_release(
        &self,
        scene: ObjectId,
        transforms: &[ObjectId],
    ) -> Result<PreparedHierarchyChange, HierarchyError> {
        self.validate_all()?;
        if !self.scene_roots.contains_key(&scene) {
            return Err(HierarchyError::MissingNode);
        }
        let selected = collect_unique(transforms)?;
        self.validate_scene_node_set(scene, &selected)?;

        let mut candidate = self.try_clone_graph()?;
        candidate.remove_selected_nodes(&selected)?;
        if candidate
            .scene_roots
            .get(&scene)
            .is_none_or(|roots| !roots.is_empty())
        {
            return Err(HierarchyError::MalformedTopology);
        }
        candidate.scene_roots.remove(&scene);
        candidate.validate_all()?;
        Ok(PreparedHierarchyChange {
            base_revision: self.revision,
            candidate,
        })
    }

    /// Prepare cleanup of an empty root sequence after scene-owned nodes are gone.
    pub fn prepare_empty_scene_root_cleanup(
        &self,
        scene: ObjectId,
    ) -> Result<PreparedHierarchyChange, HierarchyError> {
        self.validate_all()?;
        match self.scene_roots.get(&scene) {
            None => return Err(HierarchyError::MissingNode),
            Some(roots) if !roots.is_empty() => {
                return Err(HierarchyError::MalformedTopology);
            }
            Some(_) => {}
        }

        let mut candidate = self.try_clone_graph()?;
        candidate.scene_roots.remove(&scene);
        candidate.revision = self.next_revision()?;
        Ok(PreparedHierarchyChange {
            base_revision: self.revision,
            candidate,
        })
    }

    /// Commit a prepared graph replacement after checking its source revision.
    pub fn commit_change(
        &mut self,
        prepared: PreparedHierarchyChange,
    ) -> Result<(), HierarchyError> {
        self.commit_candidate(prepared.base_revision, prepared.candidate)
    }

    pub(super) fn commit_candidate(
        &mut self,
        base_revision: u64,
        candidate: SceneHierarchy,
    ) -> Result<(), HierarchyError> {
        if self.revision != base_revision {
            return Err(HierarchyError::StalePreparation);
        }
        *self = candidate;
        Ok(())
    }

    fn validate_scene_node_set(
        &self,
        scene: ObjectId,
        selected: &HashSet<ObjectId>,
    ) -> Result<(), HierarchyError> {
        for &transform in self.children.keys() {
            if self.root_owner_of_node(transform)? == RootOwner::Scene(scene)
                && !selected.contains(&transform)
            {
                return Err(HierarchyError::MalformedTopology);
            }
        }
        for &transform in selected {
            if self.root_owner_of_node(transform)? != RootOwner::Scene(scene) {
                return Err(HierarchyError::MalformedTopology);
            }
        }
        Ok(())
    }

    fn remove_selected_nodes(
        &mut self,
        selected: &HashSet<ObjectId>,
    ) -> Result<(), HierarchyError> {
        for &transform in selected {
            self.validate_local_membership(transform)?;
            let children = self
                .children
                .get(&transform)
                .ok_or(HierarchyError::MissingNode)?;
            if children.iter().any(|child| !selected.contains(child)) {
                return Err(HierarchyError::MalformedTopology);
            }
        }

        let next_revision = self.next_revision()?;
        for &transform in selected {
            if let Some(parent) = self.parents.get(&transform).copied() {
                if !selected.contains(&parent) {
                    let siblings = self
                        .children
                        .get_mut(&parent)
                        .ok_or(HierarchyError::MalformedTopology)?;
                    let position = siblings
                        .iter()
                        .position(|&sibling| sibling == transform)
                        .ok_or(HierarchyError::MalformedTopology)?;
                    siblings.remove(position);
                }
            } else if self.unowned_roots.remove(&transform) {
                // The unowned root set has no observable order.
            } else {
                let owner = self
                    .root_owners
                    .get(&transform)
                    .copied()
                    .ok_or(HierarchyError::MalformedTopology)?;
                if let RootOwner::Scene(scene) = owner {
                    let roots = self
                        .scene_roots
                        .get_mut(&scene)
                        .ok_or(HierarchyError::MalformedTopology)?;
                    let position = roots
                        .iter()
                        .position(|&root| root == transform)
                        .ok_or(HierarchyError::MalformedTopology)?;
                    roots.remove(position);
                }
            }
        }

        for &transform in selected {
            self.parents.remove(&transform);
            self.children.remove(&transform);
            self.root_owners.remove(&transform);
        }
        self.revision = next_revision;
        Ok(())
    }
}
