use super::{HierarchyError, HierarchyTarget, PreparedRegistration, RootOwner, SceneHierarchy};
use engine_identifiers::ObjectId;

impl SceneHierarchy {
    /// Reserve storage for registering an unowned root.
    pub fn prepare_unowned_root_registration(
        &mut self,
    ) -> Result<PreparedRegistration, HierarchyError> {
        self.prepare_registration(HierarchyTarget::Root(RootOwner::Unowned))
    }

    /// Reserve storage for registering a root in a scene.
    pub fn prepare_scene_root_registration(
        &mut self,
        scene: ObjectId,
    ) -> Result<PreparedRegistration, HierarchyError> {
        self.prepare_registration(HierarchyTarget::Root(RootOwner::Scene(scene)))
    }

    /// Reserve storage for registering a child under a live graph parent.
    pub fn prepare_child_registration(
        &mut self,
        parent: ObjectId,
    ) -> Result<PreparedRegistration, HierarchyError> {
        self.prepare_registration(HierarchyTarget::Child(parent))
    }

    /// Commit a prepared registration without allocating or returning a failure.
    ///
    /// The caller must supply a fresh identity and commit before mutating this graph.
    pub fn commit_registration(&mut self, prepared: PreparedRegistration, transform: ObjectId) {
        assert_eq!(
            self.revision, prepared.base_revision,
            "stale hierarchy registration"
        );
        assert!(
            !self.children.contains_key(&transform),
            "duplicate hierarchy registration"
        );
        assert!(
            !self.parents.contains_key(&transform)
                && !self.root_owners.contains_key(&transform)
                && !self.unowned_roots.contains(&transform),
            "identity already has hierarchy membership"
        );

        let PreparedRegistration {
            target,
            committed_revision,
            new_scene_roots,
            ..
        } = prepared;
        self.children.insert(transform, Vec::new());

        match target {
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
                    .expect("prepared scene root group")
                    .push(transform);
                self.root_owners.insert(transform, RootOwner::Scene(scene));
            }
            HierarchyTarget::Child(parent) => {
                self.parents.insert(transform, parent);
                self.children
                    .get_mut(&parent)
                    .expect("prepared child parent")
                    .push(transform);
            }
        }

        self.revision = committed_revision;
    }

    /// Register a root directly when no other state needs coordinated preparation.
    pub fn register_root(
        &mut self,
        transform: ObjectId,
        owner: RootOwner,
    ) -> Result<(), HierarchyError> {
        self.ensure_fresh_transform(transform)?;
        let prepared = self.prepare_registration(HierarchyTarget::Root(owner))?;
        self.commit_registration(prepared, transform);
        Ok(())
    }

    /// Register a child directly when no other state needs coordinated preparation.
    pub fn register_child(
        &mut self,
        transform: ObjectId,
        parent: ObjectId,
    ) -> Result<(), HierarchyError> {
        self.ensure_fresh_transform(transform)?;
        let prepared = self.prepare_registration(HierarchyTarget::Child(parent))?;
        self.commit_registration(prepared, transform);
        Ok(())
    }

    fn ensure_fresh_transform(&self, transform: ObjectId) -> Result<(), HierarchyError> {
        if self.children.contains_key(&transform)
            || self.parents.contains_key(&transform)
            || self.root_owners.contains_key(&transform)
            || self.unowned_roots.contains(&transform)
        {
            return Err(HierarchyError::DuplicateNode);
        }

        Ok(())
    }

    fn prepare_registration(
        &mut self,
        target: HierarchyTarget,
    ) -> Result<PreparedRegistration, HierarchyError> {
        let committed_revision = self.next_revision()?;
        if let HierarchyTarget::Child(parent) = target {
            if !self.contains(parent) {
                return Err(HierarchyError::MissingNode);
            }
            self.validate_local_membership(parent)?;
            self.validate_children_of(parent)?;
            self.children
                .get_mut(&parent)
                .ok_or(HierarchyError::MalformedTopology)?
                .try_reserve(1)
                .map_err(|_| HierarchyError::Allocation)?;
        }

        self.children
            .try_reserve(1)
            .map_err(|_| HierarchyError::Allocation)?;

        let mut new_scene_roots = None;
        match target {
            HierarchyTarget::Root(RootOwner::Unowned) => {
                self.unowned_roots
                    .try_reserve(1)
                    .map_err(|_| HierarchyError::Allocation)?;
                self.root_owners
                    .try_reserve(1)
                    .map_err(|_| HierarchyError::Allocation)?;
            }
            HierarchyTarget::Root(RootOwner::Scene(scene)) => {
                if let Some(roots) = self.scene_roots.get(&scene) {
                    self.validate_scene_roots(scene, roots)?;
                }
                self.root_owners
                    .try_reserve(1)
                    .map_err(|_| HierarchyError::Allocation)?;
                if let Some(roots) = self.scene_roots.get_mut(&scene) {
                    roots
                        .try_reserve(1)
                        .map_err(|_| HierarchyError::Allocation)?;
                } else {
                    self.scene_roots
                        .try_reserve(1)
                        .map_err(|_| HierarchyError::Allocation)?;
                    let mut roots = Vec::new();
                    roots
                        .try_reserve(1)
                        .map_err(|_| HierarchyError::Allocation)?;
                    new_scene_roots = Some(roots);
                }
            }
            HierarchyTarget::Child(_) => {
                self.parents
                    .try_reserve(1)
                    .map_err(|_| HierarchyError::Allocation)?;
            }
        }

        Ok(PreparedRegistration {
            target,
            base_revision: self.revision,
            committed_revision,
            new_scene_roots,
        })
    }

    fn validate_scene_roots(
        &self,
        scene: ObjectId,
        roots: &[ObjectId],
    ) -> Result<(), HierarchyError> {
        for (index, &root) in roots.iter().enumerate() {
            if roots[..index].contains(&root)
                || self.parents.contains_key(&root)
                || self.root_owners.get(&root) != Some(&RootOwner::Scene(scene))
                || !self.children.contains_key(&root)
            {
                return Err(HierarchyError::MalformedTopology);
            }
        }
        Ok(())
    }
}
