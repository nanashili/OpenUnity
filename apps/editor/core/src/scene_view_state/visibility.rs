//! Visibility settings and hierarchy subtree toggling.

use std::collections::BTreeSet;

use crate::scene::{EntityId, Scene};

#[derive(Default)]
pub(super) struct VisibilityState {
    pub(super) hidden: BTreeSet<EntityId>,
}

impl VisibilityState {
    pub(super) fn clear(&mut self) {
        self.hidden.clear();
    }

    pub(super) fn reconcile(&mut self, scene: &Scene) {
        self.hidden.retain(|id| scene.get(*id).is_some());
    }

    pub(super) fn is_visible(&self, id: EntityId) -> bool {
        !self.hidden.contains(&id)
    }

    pub(super) fn is_hidden(&self, id: EntityId) -> bool {
        self.hidden.contains(&id)
    }

    pub(super) fn toggle(&mut self, scene: &Scene, id: EntityId, recursive: bool) {
        toggle_subtree_state(scene, id, recursive, &mut self.hidden, None);
    }

    pub(super) fn toggle_selected(
        &mut self,
        scene: &Scene,
        selected: impl IntoIterator<Item = EntityId>,
    ) {
        toggle_selected_subtrees(scene, selected, &mut self.hidden, None);
    }
}

pub(super) fn toggle_subtree_state(
    scene: &Scene,
    id: EntityId,
    recursive: bool,
    disabled: &mut BTreeSet<EntityId>,
    allowed: Option<&BTreeSet<EntityId>>,
) {
    if scene.get(id).is_none() || allowed.is_some_and(|ids| !ids.contains(&id)) {
        return;
    }

    let targets = if recursive {
        collect_subtree(scene, id)
    } else {
        vec![id]
    };
    let target_is_hidden = disabled.contains(&id);

    for target in targets {
        if allowed.is_some_and(|ids| !ids.contains(&target)) {
            continue;
        }
        set_disabled(disabled, target, !target_is_hidden);
    }
}

pub(super) fn toggle_selected_subtrees(
    scene: &Scene,
    selected: impl IntoIterator<Item = EntityId>,
    disabled: &mut BTreeSet<EntityId>,
    allowed: Option<&BTreeSet<EntityId>>,
) {
    let selected = selected
        .into_iter()
        .filter(|id| scene.get(*id).is_some() && allowed.is_none_or(|ids| ids.contains(id)))
        .collect::<BTreeSet<_>>();

    let roots = selected
        .iter()
        .copied()
        .filter(|id| !has_selected_ancestor(scene, *id, &selected))
        .collect::<Vec<_>>();

    for root in roots {
        toggle_subtree_state(scene, root, true, disabled, allowed);
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

fn has_selected_ancestor(scene: &Scene, id: EntityId, selected: &BTreeSet<EntityId>) -> bool {
    let mut ancestor = scene.get(id).and_then(|entity| entity.parent);

    while let Some(current) = ancestor {
        if selected.contains(&current) {
            return true;
        }
        ancestor = scene.get(current).and_then(|entity| entity.parent);
    }

    false
}

fn set_disabled(disabled: &mut BTreeSet<EntityId>, id: EntityId, is_disabled: bool) {
    if is_disabled {
        disabled.insert(id);
    } else {
        disabled.remove(&id);
    }
}
