use super::{HierarchyError, HierarchyTarget, RootOwner, SceneHierarchy};
use engine_identifiers::ObjectId;

impl SceneHierarchy {
    /// Move a registered Transform to another direct parent or root group.
    pub fn reparent(
        &mut self,
        transform: ObjectId,
        target: HierarchyTarget,
    ) -> Result<(), HierarchyError> {
        let source_parent = self.validated_parent(transform)?;
        let source_owner = self.root_owner_of_node(transform)?;
        let target_owner = self.validate_target(transform, target)?;
        if source_owner != target_owner {
            return Err(HierarchyError::MalformedTopology);
        }
        if self.matches_target(transform, source_parent, target) {
            return Ok(());
        }

        let committed_revision = self.next_revision()?;
        let new_scene_roots = self.reserve_target(source_parent, target)?;
        self.unlink(transform, source_parent, source_owner)?;
        self.link(transform, target, new_scene_roots);
        self.revision = committed_revision;
        Ok(())
    }

    fn validate_target(
        &self,
        transform: ObjectId,
        target: HierarchyTarget,
    ) -> Result<RootOwner, HierarchyError> {
        match target {
            HierarchyTarget::Root(owner) => Ok(owner),
            HierarchyTarget::Child(parent) => {
                if transform == parent {
                    return Err(HierarchyError::Cycle);
                }
                self.validate_children_of(parent)?;
                self.ensure_no_cycle(transform, parent)?;
                self.root_owner_of_node(parent)
            }
        }
    }

    fn ensure_no_cycle(
        &self,
        transform: ObjectId,
        mut parent: ObjectId,
    ) -> Result<(), HierarchyError> {
        let mut traversed = 0;
        loop {
            if parent == transform {
                return Err(HierarchyError::Cycle);
            }
            let Some(ancestor) = self.parents.get(&parent).copied() else {
                return Ok(());
            };
            parent = ancestor;
            traversed += 1;
            if traversed > self.children.len() {
                return Err(HierarchyError::Cycle);
            }
        }
    }

    fn matches_target(
        &self,
        transform: ObjectId,
        source_parent: Option<ObjectId>,
        target: HierarchyTarget,
    ) -> bool {
        match target {
            HierarchyTarget::Child(parent) => source_parent == Some(parent),
            HierarchyTarget::Root(owner) => {
                source_parent.is_none() && self.root_owners.get(&transform) == Some(&owner)
            }
        }
    }

    fn reserve_target(
        &mut self,
        source_parent: Option<ObjectId>,
        target: HierarchyTarget,
    ) -> Result<Option<Vec<ObjectId>>, HierarchyError> {
        match target {
            HierarchyTarget::Child(parent) => {
                if source_parent.is_none() {
                    self.parents
                        .try_reserve(1)
                        .map_err(|_| HierarchyError::Allocation)?;
                }
                self.children
                    .get_mut(&parent)
                    .ok_or(HierarchyError::MalformedTopology)?
                    .try_reserve(1)
                    .map_err(|_| HierarchyError::Allocation)?;
                Ok(None)
            }
            HierarchyTarget::Root(RootOwner::Unowned) => {
                self.unowned_roots
                    .try_reserve(1)
                    .map_err(|_| HierarchyError::Allocation)?;
                self.root_owners
                    .try_reserve(1)
                    .map_err(|_| HierarchyError::Allocation)?;
                Ok(None)
            }
            HierarchyTarget::Root(RootOwner::Scene(scene)) => {
                self.root_owners
                    .try_reserve(1)
                    .map_err(|_| HierarchyError::Allocation)?;
                if let Some(roots) = self.scene_roots.get_mut(&scene) {
                    roots
                        .try_reserve(1)
                        .map_err(|_| HierarchyError::Allocation)?;
                    Ok(None)
                } else {
                    self.scene_roots
                        .try_reserve(1)
                        .map_err(|_| HierarchyError::Allocation)?;
                    let mut roots = Vec::new();
                    roots
                        .try_reserve(1)
                        .map_err(|_| HierarchyError::Allocation)?;
                    Ok(Some(roots))
                }
            }
        }
    }

    fn unlink(
        &mut self,
        transform: ObjectId,
        source_parent: Option<ObjectId>,
        source_owner: RootOwner,
    ) -> Result<(), HierarchyError> {
        if let Some(parent) = source_parent {
            let children = self
                .children
                .get_mut(&parent)
                .ok_or(HierarchyError::MalformedTopology)?;
            let position = children
                .iter()
                .position(|&child| child == transform)
                .ok_or(HierarchyError::MalformedTopology)?;
            children.remove(position);
            self.parents.remove(&transform);
        } else {
            match source_owner {
                RootOwner::Unowned => {
                    if !self.unowned_roots.remove(&transform) {
                        return Err(HierarchyError::MalformedTopology);
                    }
                }
                RootOwner::Scene(scene) => {
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
            self.root_owners.remove(&transform);
        }
        Ok(())
    }

    fn link(
        &mut self,
        transform: ObjectId,
        target: HierarchyTarget,
        new_scene_roots: Option<Vec<ObjectId>>,
    ) {
        match target {
            HierarchyTarget::Child(parent) => {
                self.parents.insert(transform, parent);
                self.children
                    .get_mut(&parent)
                    .expect("target capacity was prepared")
                    .push(transform);
            }
            HierarchyTarget::Root(RootOwner::Unowned) => {
                self.unowned_roots.insert(transform);
                self.root_owners.insert(transform, RootOwner::Unowned);
            }
            HierarchyTarget::Root(RootOwner::Scene(scene)) => {
                if let Some(roots) = new_scene_roots {
                    self.scene_roots.insert(scene, roots);
                }
                self.scene_roots
                    .get_mut(&scene)
                    .expect("target root capacity was prepared")
                    .push(transform);
                self.root_owners.insert(transform, RootOwner::Scene(scene));
            }
        }
    }
}
