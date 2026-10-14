//! Parent creation for a selected set of direct siblings.

use std::collections::BTreeSet;

use crate::hierarchy_revision::HierarchyRevision;

use super::super::{EntityId, Primitive, SceneError};
use super::Scene;

impl Scene {
    /// Create an identity-transform parent around selected direct siblings.
    ///
    /// The new parent takes the first selected sibling's former position.
    /// Selected children keep their local transforms and relative order.
    pub fn create_parent_for_siblings(
        &mut self,
        selected_ids: &[EntityId],
        name: impl Into<String>,
    ) -> Result<EntityId, SceneError> {
        let (parent, first_index, ordered_selection) =
            self.validate_group_selection(selected_ids)?;
        let mut candidate = self.clone();

        let new_parent = candidate.create(name, Primitive::Empty, parent)?;
        candidate.move_selection_under_parent(&ordered_selection, new_parent)?;
        candidate.set_sibling_index(new_parent, first_index)?;
        candidate.validate_for("create parent for selection")?;
        candidate.hierarchy_revision = HierarchyRevision::fresh();

        *self = candidate;
        Ok(new_parent)
    }

    fn validate_group_selection(
        &self,
        selected_ids: &[EntityId],
    ) -> Result<(Option<EntityId>, usize, Vec<EntityId>), SceneError> {
        let operation = "create parent for selection";
        if selected_ids.is_empty() {
            return Err(SceneError::new(operation, "selection is empty"));
        }

        let selected: BTreeSet<_> = selected_ids.iter().copied().collect();
        if selected.len() != selected_ids.len() {
            return Err(SceneError::new(operation, "selection contains duplicates"));
        }

        let first = self
            .entities
            .get(&selected_ids[0])
            .ok_or_else(|| SceneError::new(operation, "selection contains an unknown entity"))?;
        let parent = first.parent;
        let group = self
            .sibling_groups
            .get(&parent)
            .ok_or_else(|| SceneError::new(operation, "selected sibling group is missing"))?;
        let ordered_selection: Vec<_> = group
            .iter()
            .filter(|id| selected.contains(id))
            .copied()
            .collect();

        if ordered_selection.len() != selected.len() {
            return Err(SceneError::new(
                operation,
                "selection must contain direct siblings from one scene",
            ));
        }

        let first_index = group
            .iter()
            .position(|id| selected.contains(id))
            .ok_or_else(|| SceneError::new(operation, "selection is not in its sibling group"))?;

        Ok((parent, first_index, ordered_selection))
    }

    fn move_selection_under_parent(
        &mut self,
        selected_ids: &[EntityId],
        parent: EntityId,
    ) -> Result<(), SceneError> {
        for id in selected_ids {
            self.edit(*id, |entity| entity.parent = Some(parent))?;
        }
        Ok(())
    }
}
