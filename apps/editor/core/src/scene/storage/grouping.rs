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
        let selection = self.validate_group_selection(selected_ids)?;
        let mut candidate = self.clone();

        let new_parent = candidate.create(name, Primitive::Empty, selection.parent)?;
        candidate.apply_grouping(&selection, new_parent)?;
        candidate.validate_for("create parent for selection")?;
        candidate.hierarchy_revision = HierarchyRevision::fresh();

        *self = candidate;
        Ok(new_parent)
    }

    fn validate_group_selection(
        &self,
        selected_ids: &[EntityId],
    ) -> Result<GroupSelection, SceneError> {
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

        Ok(GroupSelection {
            parent,
            first_index,
            ordered_ids: ordered_selection,
            member_ids: selected,
        })
    }

    fn apply_grouping(
        &mut self,
        selection: &GroupSelection,
        new_parent: EntityId,
    ) -> Result<(), SceneError> {
        let operation = "create parent for selection";
        {
            let new_children = self
                .sibling_groups
                .get_mut(&Some(new_parent))
                .ok_or_else(|| SceneError::new(operation, "new parent child group is missing"))?;
            if !new_children.is_empty() {
                return Err(SceneError::new(operation, "new parent is not empty"));
            }
            new_children
                .try_reserve(selection.ordered_ids.len())
                .map_err(|error| {
                    SceneError::new(operation, format!("could not reserve child order: {error}"))
                })?;
        }

        let source_group = self
            .sibling_groups
            .get(&selection.parent)
            .ok_or_else(|| SceneError::new(operation, "selected sibling group is missing"))?;
        let selected_count = source_group
            .iter()
            .filter(|id| selection.member_ids.contains(id))
            .count();
        if selected_count != selection.ordered_ids.len() {
            return Err(SceneError::new(
                operation,
                "selection is inconsistent with its sibling group",
            ));
        }
        let new_parent_count = source_group.iter().filter(|id| **id == new_parent).count();
        if new_parent_count != 1 {
            return Err(SceneError::new(
                operation,
                "new parent is inconsistent with its sibling group",
            ));
        }
        let removed_count = selected_count + new_parent_count;
        let remaining_count = source_group
            .len()
            .checked_sub(removed_count)
            .ok_or_else(|| {
                SceneError::new(operation, "sibling group membership is inconsistent")
            })?;
        if selection.first_index > remaining_count {
            return Err(SceneError::new(
                operation,
                "new parent position is outside its sibling group",
            ));
        }

        let source_group = self
            .sibling_groups
            .get_mut(&selection.parent)
            .expect("source group was validated above");
        source_group.retain(|id| *id != new_parent && !selection.member_ids.contains(id));
        source_group.insert(selection.first_index, new_parent);

        self.sibling_groups
            .get_mut(&Some(new_parent))
            .expect("new parent group was validated above")
            .extend_from_slice(&selection.ordered_ids);

        for id in &selection.ordered_ids {
            self.entities
                .get_mut(id)
                .ok_or_else(|| SceneError::new(operation, "selected entity disappeared"))?
                .parent = Some(new_parent);
        }

        Ok(())
    }
}

struct GroupSelection {
    parent: Option<EntityId>,
    first_index: usize,
    ordered_ids: Vec<EntityId>,
    member_ids: BTreeSet<EntityId>,
}
