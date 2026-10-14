//! Editor-only Scene visibility and picking state.

use std::collections::{BTreeMap, BTreeSet};

use openunity_editor_core::scene::{EntityId, Scene};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct SceneRowState {
    pub(crate) visible: bool,
    pub(crate) visibility_mixed: bool,
    pub(crate) pickable: bool,
    pub(crate) picking_mixed: bool,
}

impl Default for SceneRowState {
    fn default() -> Self {
        Self {
            visible: true,
            visibility_mixed: false,
            pickable: true,
            picking_mixed: false,
        }
    }
}

impl SceneRowState {
    pub(crate) fn visibility_label(self) -> (&'static str, &'static str) {
        match (self.visible, self.visibility_mixed) {
            (true, false) => ("●", "Visible"),
            (true, true) => ("◐", "Visible with hidden descendants"),
            (false, false) => ("○", "Hidden"),
            (false, true) => ("◑", "Hidden with visible descendants"),
        }
    }

    pub(crate) fn picking_label(self) -> (&'static str, &'static str) {
        match (self.pickable, self.picking_mixed) {
            (true, false) => ("◆", "Pickable"),
            (true, true) => ("◇", "Pickable with unpickable descendants"),
            (false, false) => ("■", "Unpickable"),
            (false, true) => ("◈", "Unpickable with pickable descendants"),
        }
    }
}

#[derive(Default)]
pub(crate) struct SceneVisibility {
    hidden: BTreeSet<EntityId>,
    unpickable: BTreeSet<EntityId>,
}

impl SceneVisibility {
    pub(crate) fn clear(&mut self) {
        self.hidden.clear();
        self.unpickable.clear();
    }

    pub(crate) fn reconcile(&mut self, scene: &Scene) {
        self.hidden.retain(|id| scene.get(*id).is_some());
        self.unpickable.retain(|id| scene.get(*id).is_some());
    }

    pub(crate) fn is_visible(&self, id: EntityId) -> bool {
        !self.hidden.contains(&id)
    }

    pub(crate) fn is_pickable(&self, id: EntityId) -> bool {
        !self.unpickable.contains(&id)
    }

    pub(crate) fn toggle_visibility(&mut self, scene: &Scene, id: EntityId, recursive: bool) {
        toggle_subtree_state(scene, id, recursive, &mut self.hidden);
    }

    pub(crate) fn toggle_pickability(&mut self, scene: &Scene, id: EntityId, recursive: bool) {
        toggle_subtree_state(scene, id, recursive, &mut self.unpickable);
    }

    pub(crate) fn row_states(&self, scene: &Scene) -> BTreeMap<EntityId, SceneRowState> {
        let mut counts = BTreeMap::<EntityId, RowStateCounts>::new();
        let mut states = BTreeMap::new();
        let mut pending = Vec::new();

        if let Some(roots) = scene.sibling_group(None) {
            pending.extend(roots.iter().rev().map(|id| (*id, false)));
        }

        while let Some((id, visited)) = pending.pop() {
            if !visited {
                pending.push((id, true));
                if let Some(children) = scene.sibling_group(Some(id)) {
                    pending.extend(children.iter().rev().map(|child| (*child, false)));
                }
                continue;
            }

            let own = RowStateCounts::for_entity(self, id);
            let mut subtree = own;

            if let Some(children) = scene.sibling_group(Some(id)) {
                for child in children {
                    if let Some(child_counts) = counts.get(child) {
                        subtree.add(*child_counts);
                    }
                }
            }

            counts.insert(id, subtree);
            states.insert(id, own.into_row_state(subtree));
        }

        states
    }
}

fn toggle_subtree_state(
    scene: &Scene,
    id: EntityId,
    recursive: bool,
    disabled: &mut BTreeSet<EntityId>,
) {
    if scene.get(id).is_none() {
        return;
    }

    let targets = if recursive {
        collect_subtree(scene, id)
    } else {
        vec![id]
    };
    let target_is_enabled = disabled.contains(&id);

    for target in targets {
        if target_is_enabled {
            disabled.remove(&target);
        } else {
            disabled.insert(target);
        }
    }
}

fn collect_subtree(scene: &Scene, root: EntityId) -> Vec<EntityId> {
    let mut result = Vec::new();
    let mut pending = vec![root];

    while let Some(id) = pending.pop() {
        result.push(id);
        if let Some(children) = scene.sibling_group(Some(id)) {
            pending.extend(children.iter().copied());
        }
    }

    result
}

#[derive(Clone, Copy, Default)]
struct RowStateCounts {
    visible: usize,
    hidden: usize,
    pickable: usize,
    unpickable: usize,
}

impl RowStateCounts {
    fn for_entity(state: &SceneVisibility, id: EntityId) -> Self {
        Self {
            visible: usize::from(state.is_visible(id)),
            hidden: usize::from(!state.is_visible(id)),
            pickable: usize::from(state.is_pickable(id)),
            unpickable: usize::from(!state.is_pickable(id)),
        }
    }

    fn add(&mut self, other: Self) {
        self.visible = self.visible.saturating_add(other.visible);
        self.hidden = self.hidden.saturating_add(other.hidden);
        self.pickable = self.pickable.saturating_add(other.pickable);
        self.unpickable = self.unpickable.saturating_add(other.unpickable);
    }

    fn into_row_state(self, subtree: Self) -> SceneRowState {
        SceneRowState {
            visible: self.visible == 1,
            visibility_mixed: subtree.visible > 0 && subtree.hidden > 0,
            pickable: self.pickable == 1,
            picking_mixed: subtree.pickable > 0 && subtree.unpickable > 0,
        }
    }
}
