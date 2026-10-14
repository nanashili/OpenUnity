//! In-memory Scene-view visibility, independent from authored scene data.

use std::collections::{BTreeMap, BTreeSet};

use openunity_editor_core::scene::{EntityId, Scene};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct VisibilityRowState {
    pub(crate) visible: bool,
    pub(crate) mixed_descendants: bool,
}

impl Default for VisibilityRowState {
    fn default() -> Self {
        Self {
            visible: true,
            mixed_descendants: false,
        }
    }
}

impl VisibilityRowState {
    pub(crate) fn control_label(self) -> (&'static str, &'static str) {
        match (self.visible, self.mixed_descendants) {
            (true, false) => ("●", "Visible"),
            (true, true) => ("◐", "Visible with hidden descendants"),
            (false, false) => ("○", "Hidden"),
            (false, true) => ("◑", "Hidden with visible descendants"),
        }
    }
}

#[derive(Default)]
pub(crate) struct SceneVisibility {
    hidden: BTreeSet<EntityId>,
}

impl SceneVisibility {
    pub(crate) fn clear(&mut self) {
        self.hidden.clear();
    }

    pub(crate) fn reconcile(&mut self, scene: &Scene) {
        self.hidden.retain(|id| scene.get(*id).is_some());
    }

    pub(crate) fn is_visible(&self, id: EntityId) -> bool {
        !self.hidden.contains(&id)
    }

    pub(crate) fn toggle(&mut self, scene: &Scene, id: EntityId, recursive: bool) {
        if scene.get(id).is_none() {
            return;
        }

        let targets = if recursive {
            collect_subtree(scene, id)
        } else {
            vec![id]
        };
        let make_visible = !self.is_visible(id);

        for target in targets {
            if make_visible {
                self.hidden.remove(&target);
            } else {
                self.hidden.insert(target);
            }
        }
    }

    pub(crate) fn row_states(&self, scene: &Scene) -> BTreeMap<EntityId, VisibilityRowState> {
        let mut totals = BTreeMap::<EntityId, (usize, usize)>::new();
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

            let visible = usize::from(self.is_visible(id));
            let hidden = usize::from(!self.is_visible(id));
            let mut visible_count = visible;
            let mut hidden_count = hidden;

            if let Some(children) = scene.sibling_group(Some(id)) {
                for child in children {
                    if let Some((child_visible, child_hidden)) = totals.get(child) {
                        visible_count = visible_count.saturating_add(*child_visible);
                        hidden_count = hidden_count.saturating_add(*child_hidden);
                    }
                }
            }

            totals.insert(id, (visible_count, hidden_count));
            states.insert(
                id,
                VisibilityRowState {
                    visible: visible == 1,
                    mixed_descendants: visible_count > 0 && hidden_count > 0,
                },
            );
        }

        states
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
