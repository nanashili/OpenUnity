use super::{HierarchyError, RootOwner, SceneHierarchy};
use engine_identifiers::ObjectId;
use std::collections::HashSet;

impl SceneHierarchy {
    /// Validate the target membership and return its direct parent, if any.
    pub fn validated_parent(
        &self,
        transform: ObjectId,
    ) -> Result<Option<ObjectId>, HierarchyError> {
        if !self.contains(transform) {
            return Err(HierarchyError::MissingNode);
        }

        match self.parents.get(&transform).copied() {
            Some(_) => self.validate_parented_group(transform).map(Some),
            None => {
                self.validate_root_membership(transform)?;
                Ok(None)
            }
        }
    }

    /// Validate a parent and its direct child sequence without walking ancestors.
    pub fn validate_parented_group(&self, transform: ObjectId) -> Result<ObjectId, HierarchyError> {
        let parent = self
            .parents
            .get(&transform)
            .copied()
            .ok_or(HierarchyError::RootReorderingUnsupported)?;
        self.validate_local_membership(transform)?;
        if self.validate_children_of(parent)?.is_empty() {
            return Err(HierarchyError::MalformedTopology);
        }

        Ok(parent)
    }

    pub(super) fn validate_children_of(
        &self,
        parent: ObjectId,
    ) -> Result<&[ObjectId], HierarchyError> {
        let children = self
            .children
            .get(&parent)
            .ok_or(HierarchyError::MalformedTopology)?;
        self.validate_local_membership(parent)?;

        let mut unique_children = HashSet::new();
        unique_children
            .try_reserve(children.len())
            .map_err(|_| HierarchyError::Allocation)?;
        for &child in children {
            if !unique_children.insert(child) || !self.children.contains_key(&child) {
                return Err(HierarchyError::MalformedTopology);
            }
            if self.parents.get(&child) != Some(&parent)
                || self.root_owners.contains_key(&child)
                || self.unowned_roots.contains(&child)
            {
                return Err(HierarchyError::MalformedTopology);
            }
        }
        Ok(children)
    }

    /// Validate the complete ordered root group for a published scene.
    pub fn validate_scene_root_group(
        &self,
        scene: ObjectId,
        target: ObjectId,
    ) -> Result<&[ObjectId], HierarchyError> {
        if !self.contains(target) {
            return Err(HierarchyError::MissingNode);
        }
        if self.parents.contains_key(&target)
            || self.root_owners.get(&target) != Some(&RootOwner::Scene(scene))
        {
            return Err(HierarchyError::MalformedTopology);
        }

        let roots = self
            .scene_roots
            .get(&scene)
            .ok_or(HierarchyError::MalformedTopology)?;
        self.validate_scene_roots(scene, roots)?;
        if occurrence_count(roots, target) != 1 {
            return Err(HierarchyError::MalformedTopology);
        }

        Ok(roots)
    }

    pub(super) fn validate_local_membership(
        &self,
        transform: ObjectId,
    ) -> Result<(), HierarchyError> {
        if !self.children.contains_key(&transform) {
            return Err(HierarchyError::MissingNode);
        }

        if let Some(parent) = self.parents.get(&transform).copied() {
            if self.root_owners.contains_key(&transform)
                || self.unowned_roots.contains(&transform)
                || self.child_occurrences(parent, transform) != 1
            {
                return Err(HierarchyError::MalformedTopology);
            }
            if !self.children.contains_key(&parent) {
                return Err(HierarchyError::MalformedTopology);
            }
            return Ok(());
        }

        self.validate_root_membership(transform)
    }

    fn validate_root_membership(&self, transform: ObjectId) -> Result<(), HierarchyError> {
        match self.root_owners.get(&transform).copied() {
            Some(RootOwner::Unowned) => {
                if !self.unowned_roots.contains(&transform) {
                    return Err(HierarchyError::MalformedTopology);
                }
            }
            Some(RootOwner::Scene(scene)) => {
                if self.unowned_roots.contains(&transform)
                    || self
                        .scene_roots
                        .get(&scene)
                        .is_none_or(|roots| occurrence_count(roots, transform) != 1)
                {
                    return Err(HierarchyError::MalformedTopology);
                }
            }
            None => return Err(HierarchyError::MalformedTopology),
        }
        Ok(())
    }

    pub(super) fn root_owner_of_node(
        &self,
        transform: ObjectId,
    ) -> Result<RootOwner, HierarchyError> {
        let mut current = transform;
        let mut traversed = 0;

        loop {
            self.validate_local_membership(current)?;
            if let Some(parent) = self.parents.get(&current).copied() {
                current = parent;
                traversed += 1;
                if traversed > self.children.len() {
                    return Err(HierarchyError::Cycle);
                }
                continue;
            }
            return self
                .root_owners
                .get(&current)
                .copied()
                .ok_or(HierarchyError::MalformedTopology);
        }
    }

    pub(super) fn validate_all(&self) -> Result<(), HierarchyError> {
        if self.parents.len() > self.children.len()
            || self.root_owners.len() + self.parents.len() != self.children.len()
        {
            return Err(HierarchyError::MalformedTopology);
        }

        for (&transform, children) in &self.children {
            self.validate_local_membership(transform)?;
            let mut unique_children = HashSet::new();
            unique_children
                .try_reserve(children.len())
                .map_err(|_| HierarchyError::Allocation)?;
            for &child in children {
                if !unique_children.insert(child)
                    || !self.children.contains_key(&child)
                    || self.parents.get(&child) != Some(&transform)
                {
                    return Err(HierarchyError::MalformedTopology);
                }
            }
            self.root_owner_of_node(transform)?;
        }

        for (&scene, roots) in &self.scene_roots {
            self.validate_scene_roots(scene, roots)?;
        }

        for &root in &self.unowned_roots {
            if self.parents.contains_key(&root)
                || self.root_owners.get(&root) != Some(&RootOwner::Unowned)
            {
                return Err(HierarchyError::MalformedTopology);
            }
        }
        Ok(())
    }

    pub(super) fn child_occurrences(&self, parent: ObjectId, child: ObjectId) -> usize {
        self.children
            .get(&parent)
            .map_or(0, |children| occurrence_count(children, child))
    }

    pub(super) fn validate_scene_roots(
        &self,
        scene: ObjectId,
        roots: &[ObjectId],
    ) -> Result<(), HierarchyError> {
        let mut unique_roots = HashSet::new();
        unique_roots
            .try_reserve(roots.len())
            .map_err(|_| HierarchyError::Allocation)?;

        for &root in roots {
            if !unique_roots.insert(root)
                || !self.children.contains_key(&root)
                || self.parents.contains_key(&root)
                || self.root_owners.get(&root) != Some(&RootOwner::Scene(scene))
                || self.unowned_roots.contains(&root)
            {
                return Err(HierarchyError::MalformedTopology);
            }
        }

        Ok(())
    }
}

fn occurrence_count(items: &[ObjectId], target: ObjectId) -> usize {
    items.iter().filter(|&&item| item == target).count()
}
