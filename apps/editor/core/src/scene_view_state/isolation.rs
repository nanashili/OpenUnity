//! Fixed isolation roots and membership derived from the live scene graph.

use std::collections::BTreeSet;

use crate::hierarchy::HierarchyRevision;
use crate::scene::{EntityId, Scene};

pub(super) struct IsolationState {
    pub(super) saved_hidden: BTreeSet<EntityId>,
    pub(super) temporary_hidden: BTreeSet<EntityId>,
    pub(super) roots: BTreeSet<EntityId>,
    pub(super) members: BTreeSet<EntityId>,
    hierarchy_revision: HierarchyRevision,
}

impl IsolationState {
    pub(super) fn new(
        scene: &Scene,
        roots: BTreeSet<EntityId>,
        saved_hidden: BTreeSet<EntityId>,
    ) -> Self {
        Self {
            saved_hidden,
            temporary_hidden: BTreeSet::new(),
            members: collect_members(scene, &roots),
            roots,
            hierarchy_revision: scene.hierarchy_revision(),
        }
    }

    pub(super) fn reconcile_ids(&mut self, scene: &Scene) {
        self.saved_hidden.retain(|id| scene.get(*id).is_some());
        self.temporary_hidden.retain(|id| scene.get(*id).is_some());
        self.roots.retain(|id| scene.get(*id).is_some());
    }

    pub(super) fn refresh_members_if_needed(&mut self, scene: &Scene) {
        let current_revision = scene.hierarchy_revision();
        if self.hierarchy_revision == current_revision {
            return;
        }

        self.members = collect_members(scene, &self.roots);
        self.hierarchy_revision = current_revision;
    }
}

fn collect_members(scene: &Scene, roots: &BTreeSet<EntityId>) -> BTreeSet<EntityId> {
    let mut members = BTreeSet::new();
    let mut pending = roots.iter().copied().collect::<Vec<_>>();

    while let Some(id) = pending.pop() {
        if !members.insert(id) {
            continue;
        }

        if let Some(children) = scene.sibling_group(Some(id)) {
            pending.extend(children.iter().copied());
        }
    }

    members
}
