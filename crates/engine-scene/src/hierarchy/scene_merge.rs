use super::unique::ensure_unique;
use super::{HierarchyError, PreparedSceneMerge, RootOwner, SceneHierarchy, graph::clone_ids};
use engine_identifiers::ObjectId;

impl SceneHierarchy {
    /// Build a private merged graph for atomic publication of an unpublished scene.
    pub fn prepare_merge(
        &self,
        scene: ObjectId,
        staged: &SceneHierarchy,
        staged_roots: &[ObjectId],
    ) -> Result<PreparedSceneMerge, HierarchyError> {
        self.validate_all()?;
        staged.validate_all()?;
        self.validate_staged_roots(staged, scene, staged_roots)?;

        let next_revision = self.next_revision()?;
        let mut candidate = self.try_clone_graph()?;
        candidate
            .children
            .try_reserve(staged.children.len())
            .map_err(|_| HierarchyError::Allocation)?;
        candidate
            .parents
            .try_reserve(staged.parents.len())
            .map_err(|_| HierarchyError::Allocation)?;
        candidate
            .root_owners
            .try_reserve(staged_roots.len())
            .map_err(|_| HierarchyError::Allocation)?;
        candidate
            .scene_roots
            .try_reserve(1)
            .map_err(|_| HierarchyError::Allocation)?;

        let roots = clone_ids(staged_roots)?;
        for (&transform, children) in &staged.children {
            if candidate.children.contains_key(&transform) {
                return Err(HierarchyError::DuplicateNode);
            }
            candidate.children.insert(transform, clone_ids(children)?);
        }
        for (&transform, &parent) in &staged.parents {
            candidate.parents.insert(transform, parent);
        }
        for &root in staged_roots {
            candidate.unowned_roots.remove(&root);
            candidate.root_owners.insert(root, RootOwner::Scene(scene));
        }
        candidate.scene_roots.insert(scene, roots);
        candidate.revision = next_revision;
        candidate.validate_all()?;

        Ok(PreparedSceneMerge {
            base_revision: self.revision,
            candidate,
        })
    }

    /// Publish a previously prepared scene graph if the host topology is unchanged.
    pub fn commit_merge(&mut self, prepared: PreparedSceneMerge) -> Result<(), HierarchyError> {
        self.commit_candidate(prepared.base_revision, prepared.candidate)
    }

    fn validate_staged_roots(
        &self,
        staged: &SceneHierarchy,
        scene: ObjectId,
        staged_roots: &[ObjectId],
    ) -> Result<(), HierarchyError> {
        if self.scene_roots.contains_key(&scene) {
            return Err(HierarchyError::DuplicateNode);
        }
        if !staged.scene_roots.is_empty() || staged_roots.len() != staged.unowned_roots.len() {
            return Err(HierarchyError::MalformedTopology);
        }
        ensure_unique(staged_roots)?;
        for &root in staged_roots {
            if !staged.unowned_roots.contains(&root)
                || staged.root_owners.get(&root) != Some(&RootOwner::Unowned)
                || staged.parents.contains_key(&root)
            {
                return Err(HierarchyError::MalformedTopology);
            }
        }
        for &root in &staged.unowned_roots {
            if !staged_roots.contains(&root) {
                return Err(HierarchyError::MalformedTopology);
            }
        }
        Ok(())
    }
}
