//! Picking state and its hierarchy subtree operations.

use std::collections::BTreeSet;

use crate::scene::{EntityId, Scene};

use super::visibility::{toggle_selected_subtrees, toggle_subtree_state};

#[derive(Default)]
pub(super) struct PickingState {
    unpickable: BTreeSet<EntityId>,
}

impl PickingState {
    pub(super) fn clear(&mut self) {
        self.unpickable.clear();
    }

    pub(super) fn reconcile(&mut self, scene: &Scene) {
        self.unpickable.retain(|id| scene.get(*id).is_some());
    }

    pub(super) fn is_pickable(&self, id: EntityId) -> bool {
        !self.unpickable.contains(&id)
    }

    pub(super) fn toggle(&mut self, scene: &Scene, id: EntityId, recursive: bool) {
        toggle_subtree_state(scene, id, recursive, &mut self.unpickable, None);
    }

    pub(super) fn toggle_selected(
        &mut self,
        scene: &Scene,
        selected: impl IntoIterator<Item = EntityId>,
    ) {
        toggle_selected_subtrees(scene, selected, &mut self.unpickable, None);
    }
}
