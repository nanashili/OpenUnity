use std::collections::HashSet;

use openunity_editor_core::scene::{EntityId, MAX_HIERARCHY_DEPTH, Scene};

/// View-only authored hierarchy branch visibility.
#[derive(Default)]
pub struct ExpansionState {
    collapsed: HashSet<EntityId>,
}

impl ExpansionState {
    pub(crate) fn clear(&mut self) {
        self.collapsed.clear();
    }

    pub(crate) fn reconcile(&mut self, scene: &Scene) {
        let renderable_branches = renderable_branches(scene);
        self.collapsed
            .retain(|entity_id| renderable_branches.contains(entity_id));
    }

    pub(super) fn is_collapsed(&self, entity_id: EntityId) -> bool {
        self.collapsed.contains(&entity_id)
    }

    pub(super) fn toggle(&mut self, scene: &Scene, entity_id: EntityId, recursive: bool) {
        let should_collapse = !self.is_collapsed(entity_id);

        if recursive {
            self.set_subtree_collapsed(scene, entity_id, should_collapse);
        } else {
            self.set_collapsed(entity_id, should_collapse);
        }
    }

    fn set_subtree_collapsed(&mut self, scene: &Scene, root_id: EntityId, should_collapse: bool) {
        for entity_id in renderable_branch_subtree(scene, root_id) {
            self.set_collapsed(entity_id, should_collapse);
        }
    }

    fn set_collapsed(&mut self, entity_id: EntityId, should_collapse: bool) {
        if should_collapse {
            self.collapsed.insert(entity_id);
        } else {
            self.collapsed.remove(&entity_id);
        }
    }
}

pub(super) fn has_renderable_children(scene: &Scene, entity_id: EntityId, depth: usize) -> bool {
    depth + 1 < MAX_HIERARCHY_DEPTH
        && scene
            .sibling_group(Some(entity_id))
            .is_some_and(|children| !children.is_empty())
}

fn renderable_branches(scene: &Scene) -> HashSet<EntityId> {
    let mut branches = HashSet::new();
    let Some(roots) = scene.sibling_group(None) else {
        return branches;
    };

    let mut pending: Vec<_> = roots
        .iter()
        .rev()
        .map(|entity_id| (*entity_id, 0))
        .collect();
    while let Some((entity_id, depth)) = pending.pop() {
        if !has_renderable_children(scene, entity_id, depth) {
            continue;
        }

        branches.insert(entity_id);
        if let Some(children) = scene.sibling_group(Some(entity_id)) {
            pending.extend(children.iter().rev().map(|child_id| (*child_id, depth + 1)));
        }
    }

    branches
}

fn renderable_branch_subtree(scene: &Scene, root_id: EntityId) -> Vec<EntityId> {
    let Some(depth) = entity_depth(scene, root_id) else {
        return Vec::new();
    };

    if !has_renderable_children(scene, root_id, depth) {
        return Vec::new();
    }

    let mut branches = Vec::new();
    let mut pending = vec![(root_id, depth)];
    while let Some((entity_id, current_depth)) = pending.pop() {
        if !has_renderable_children(scene, entity_id, current_depth) {
            continue;
        }

        branches.push(entity_id);
        if let Some(children) = scene.sibling_group(Some(entity_id)) {
            pending.extend(
                children
                    .iter()
                    .rev()
                    .map(|child_id| (*child_id, current_depth + 1)),
            );
        }
    }

    branches
}

fn entity_depth(scene: &Scene, entity_id: EntityId) -> Option<usize> {
    let mut depth = 0;
    let mut current = scene.get(entity_id)?;
    while let Some(parent_id) = current.parent {
        depth += 1;
        current = scene.get(parent_id)?;
    }
    Some(depth)
}
