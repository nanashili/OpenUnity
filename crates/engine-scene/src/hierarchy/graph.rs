use super::{HierarchyError, RootOwner};
use engine_identifiers::ObjectId;
use std::collections::{HashMap, HashSet};

/// Opaque owner of parent links, ordered child groups, and root groups.
#[derive(Debug, Default)]
pub struct SceneHierarchy {
    pub(super) parents: HashMap<ObjectId, ObjectId>,
    pub(super) children: HashMap<ObjectId, Vec<ObjectId>>,
    pub(super) scene_roots: HashMap<ObjectId, Vec<ObjectId>>,
    pub(super) unowned_roots: HashSet<ObjectId>,
    pub(super) root_owners: HashMap<ObjectId, RootOwner>,
    pub(super) revision: u64,
}

impl SceneHierarchy {
    /// Whether this graph has storage for the Transform.
    pub fn contains(&self, transform: ObjectId) -> bool {
        self.children.contains_key(&transform)
    }

    /// Return the recorded direct parent without validating graph membership.
    pub fn parent_of(&self, transform: ObjectId) -> Option<ObjectId> {
        self.parents.get(&transform).copied()
    }

    /// Return the ordered direct children of a registered Transform.
    pub fn children(&self, transform: ObjectId) -> Option<&[ObjectId]> {
        self.children.get(&transform).map(Vec::as_slice)
    }

    /// Return the authored root order for a published scene.
    pub fn scene_roots(&self, scene: ObjectId) -> Option<&[ObjectId]> {
        self.scene_roots.get(&scene).map(Vec::as_slice)
    }

    /// Whether a parentless Transform belongs to the unowned-root set.
    pub fn is_unowned_root(&self, transform: ObjectId) -> bool {
        self.unowned_roots.contains(&transform)
    }

    /// Return the stored owner when a Transform is a root.
    pub fn root_owner(&self, transform: ObjectId) -> Option<RootOwner> {
        self.root_owners.get(&transform).copied()
    }

    pub(super) fn next_revision(&self) -> Result<u64, HierarchyError> {
        self.revision
            .checked_add(1)
            .ok_or(HierarchyError::Allocation)
    }

    pub(super) fn try_clone_graph(&self) -> Result<Self, HierarchyError> {
        let mut cloned = Self {
            parents: HashMap::new(),
            children: HashMap::new(),
            scene_roots: HashMap::new(),
            unowned_roots: HashSet::new(),
            root_owners: HashMap::new(),
            revision: self.revision,
        };
        cloned
            .parents
            .try_reserve(self.parents.len())
            .map_err(|_| HierarchyError::Allocation)?;
        cloned
            .children
            .try_reserve(self.children.len())
            .map_err(|_| HierarchyError::Allocation)?;
        cloned
            .scene_roots
            .try_reserve(self.scene_roots.len())
            .map_err(|_| HierarchyError::Allocation)?;
        cloned
            .unowned_roots
            .try_reserve(self.unowned_roots.len())
            .map_err(|_| HierarchyError::Allocation)?;
        cloned
            .root_owners
            .try_reserve(self.root_owners.len())
            .map_err(|_| HierarchyError::Allocation)?;

        for (&transform, &parent) in &self.parents {
            cloned.parents.insert(transform, parent);
        }
        for (&transform, children) in &self.children {
            cloned.children.insert(transform, clone_ids(children)?);
        }
        for (&scene, roots) in &self.scene_roots {
            cloned.scene_roots.insert(scene, clone_ids(roots)?);
        }
        cloned
            .unowned_roots
            .extend(self.unowned_roots.iter().copied());
        cloned
            .root_owners
            .extend(self.root_owners.iter().map(|(id, owner)| (*id, *owner)));

        Ok(cloned)
    }
}

pub(super) fn clone_ids(ids: &[ObjectId]) -> Result<Vec<ObjectId>, HierarchyError> {
    let mut copied = Vec::new();
    copied
        .try_reserve(ids.len())
        .map_err(|_| HierarchyError::Allocation)?;
    copied.extend_from_slice(ids);
    Ok(copied)
}
