//! Transient editor-only visibility, picking, and isolation state.

mod isolation;
mod picking;
mod row_summary;
mod visibility;

pub use row_summary::SceneRowState;

use std::collections::{BTreeMap, BTreeSet};

use crate::scene::{EntityId, Scene};

use isolation::IsolationState;
use picking::PickingState;
use visibility::VisibilityState;

/// Document-scoped view state that is excluded from authored scenes and history.
#[derive(Default)]
pub struct SceneViewState {
    visibility: VisibilityState,
    picking: PickingState,
    isolation: Option<IsolationState>,
}

impl SceneViewState {
    /// Clear all view state when the editor replaces its document.
    pub fn clear(&mut self) {
        self.visibility.clear();
        self.picking.clear();
        self.isolation = None;
    }

    /// Prune deleted IDs and refresh isolated membership after hierarchy edits.
    pub fn reconcile(&mut self, scene: &Scene) {
        self.visibility.reconcile(scene);
        self.picking.reconcile(scene);

        let isolation = self.isolation.as_mut();
        let Some(isolation) = isolation else {
            return;
        };

        isolation.reconcile_ids(scene);
        if isolation.roots.is_empty() {
            self.exit_isolation(scene);
            return;
        }

        isolation.refresh_members_if_needed(scene);
    }

    /// Whether an entity should be drawn in the authored viewport.
    pub fn is_visible(&self, id: EntityId) -> bool {
        match &self.isolation {
            Some(isolation) => {
                isolation.members.contains(&id) && !isolation.temporary_hidden.contains(&id)
            }
            None => self.visibility.is_visible(id),
        }
    }

    /// Whether viewport hit testing may select an entity.
    pub fn is_pickable(&self, id: EntityId) -> bool {
        self.picking.is_pickable(id)
    }

    /// Whether an isolation session is active.
    pub fn is_isolation_active(&self) -> bool {
        self.isolation.is_some()
    }

    /// Enter isolation around the current selection; returns false for no live roots.
    pub fn enter_isolation(
        &mut self,
        scene: &Scene,
        selected: impl IntoIterator<Item = EntityId>,
    ) -> bool {
        if self.isolation.is_some() {
            return false;
        }

        let roots = selected
            .into_iter()
            .filter(|id| scene.get(*id).is_some())
            .collect::<BTreeSet<_>>();
        if roots.is_empty() {
            return false;
        }

        self.isolation = Some(IsolationState::new(
            scene,
            roots,
            self.visibility.hidden.clone(),
        ));
        true
    }

    /// Exit isolation and restore the visibility snapshot for surviving entities.
    pub fn exit_isolation(&mut self, scene: &Scene) {
        let Some(isolation) = self.isolation.take() else {
            return;
        };

        self.visibility.hidden = isolation
            .saved_hidden
            .into_iter()
            .filter(|id| scene.get(*id).is_some())
            .collect();
    }

    /// Toggle visibility for one hierarchy row and optionally its descendants.
    pub fn toggle_visibility(&mut self, scene: &Scene, id: EntityId, recursive: bool) {
        if let Some(isolation) = self.isolation.as_mut() {
            visibility::toggle_subtree_state(
                scene,
                id,
                recursive,
                &mut isolation.temporary_hidden,
                Some(&isolation.members),
            );
        } else {
            self.visibility.toggle(scene, id, recursive);
        }
    }

    /// Toggle visibility for selected roots and their descendants.
    pub fn toggle_visibility_for_selection(
        &mut self,
        scene: &Scene,
        selected: impl IntoIterator<Item = EntityId>,
    ) {
        if let Some(isolation) = self.isolation.as_mut() {
            visibility::toggle_selected_subtrees(
                scene,
                selected,
                &mut isolation.temporary_hidden,
                Some(&isolation.members),
            );
        } else {
            self.visibility.toggle_selected(scene, selected);
        }
    }

    /// Toggle pickability for one hierarchy row and optionally its descendants.
    pub fn toggle_pickability(&mut self, scene: &Scene, id: EntityId, recursive: bool) {
        self.picking.toggle(scene, id, recursive);
    }

    /// Toggle pickability for selected roots and their descendants.
    pub fn toggle_pickability_for_selection(
        &mut self,
        scene: &Scene,
        selected: impl IntoIterator<Item = EntityId>,
    ) {
        self.picking.toggle_selected(scene, selected);
    }

    /// Describe stored visibility and picking settings for hierarchy rows.
    pub fn row_states(&self, scene: &Scene) -> BTreeMap<EntityId, SceneRowState> {
        row_summary::row_states(scene, self)
    }

    fn is_hidden_for_row(&self, id: EntityId) -> bool {
        match &self.isolation {
            Some(isolation) if isolation.members.contains(&id) => {
                isolation.temporary_hidden.contains(&id)
            }
            _ => self.visibility.is_hidden(id),
        }
    }

    fn can_edit_visibility(&self, id: EntityId) -> bool {
        self.isolation
            .as_ref()
            .is_none_or(|isolation| isolation.members.contains(&id))
    }
}
