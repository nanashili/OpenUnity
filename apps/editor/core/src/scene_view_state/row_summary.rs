//! Hierarchy row visibility and picking summaries.

use std::collections::BTreeMap;

use crate::scene::{EntityId, Scene};

use super::SceneViewState;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
/// Visibility and picking summary used by one hierarchy row.
pub struct SceneRowState {
    /// Whether the entity's stored visibility setting is visible.
    pub visible: bool,
    /// Whether this row's visibility control can change the active view state.
    pub visibility_enabled: bool,
    /// Whether descendants have mixed visibility settings.
    pub visibility_mixed: bool,
    /// Whether the entity can be picked in the viewport.
    pub pickable: bool,
    /// Whether descendants have mixed picking settings.
    pub picking_mixed: bool,
}

impl Default for SceneRowState {
    fn default() -> Self {
        Self {
            visible: true,
            visibility_enabled: true,
            visibility_mixed: false,
            pickable: true,
            picking_mixed: false,
        }
    }
}

impl SceneRowState {
    /// Return the visibility glyph and accessible label for this row.
    pub fn visibility_label(self) -> (&'static str, &'static str) {
        match (self.visible, self.visibility_mixed) {
            (true, false) => ("●", "Visible"),
            (true, true) => ("◐", "Visible with hidden descendants"),
            (false, false) => ("○", "Hidden"),
            (false, true) => ("◑", "Hidden with visible descendants"),
        }
    }

    /// Return the picking glyph and accessible label for this row.
    pub fn picking_label(self) -> (&'static str, &'static str) {
        match (self.pickable, self.picking_mixed) {
            (true, false) => ("◆", "Pickable"),
            (true, true) => ("◇", "Pickable with unpickable descendants"),
            (false, false) => ("■", "Unpickable"),
            (false, true) => ("◈", "Unpickable with pickable descendants"),
        }
    }
}

pub(super) fn row_states(
    scene: &Scene,
    state: &SceneViewState,
) -> BTreeMap<EntityId, SceneRowState> {
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

        let own = RowStateCounts::for_entity(state, id);
        let mut subtree = own;
        if let Some(children) = scene.sibling_group(Some(id)) {
            for child in children {
                if let Some(child_counts) = counts.get(child) {
                    subtree.add(*child_counts);
                }
            }
        }

        counts.insert(id, subtree);
        states.insert(
            id,
            own.into_row_state(subtree, state.can_edit_visibility(id)),
        );
    }

    states
}

#[derive(Clone, Copy, Default)]
struct RowStateCounts {
    visible: usize,
    hidden: usize,
    pickable: usize,
    unpickable: usize,
}

impl RowStateCounts {
    fn for_entity(state: &SceneViewState, id: EntityId) -> Self {
        let visible = !state.is_hidden_for_row(id);
        let pickable = state.is_pickable(id);
        Self {
            visible: usize::from(visible),
            hidden: usize::from(!visible),
            pickable: usize::from(pickable),
            unpickable: usize::from(!pickable),
        }
    }

    fn add(&mut self, other: Self) {
        self.visible = self.visible.saturating_add(other.visible);
        self.hidden = self.hidden.saturating_add(other.hidden);
        self.pickable = self.pickable.saturating_add(other.pickable);
        self.unpickable = self.unpickable.saturating_add(other.unpickable);
    }

    fn into_row_state(self, subtree: Self, visibility_enabled: bool) -> SceneRowState {
        SceneRowState {
            visible: self.visible == 1,
            visibility_enabled,
            visibility_mixed: subtree.visible > 0 && subtree.hidden > 0,
            pickable: self.pickable == 1,
            picking_mixed: subtree.pickable > 0 && subtree.unpickable > 0,
        }
    }
}
