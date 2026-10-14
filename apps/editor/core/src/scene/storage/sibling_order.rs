//! Ordered root and direct-child groups for authored scenes.

use std::collections::BTreeMap;

use crate::hierarchy_revision::HierarchyRevision;

use super::super::{Entity, EntityId, SceneError};
use super::Scene;

impl Scene {
    pub(crate) fn from_ordered_entities(
        name: impl Into<String>,
        next_id: u64,
        ordered_entities: Vec<(Entity, usize)>,
    ) -> Result<Self, SceneError> {
        let operation = "load ordered scene";
        let entities = ordered_entities
            .iter()
            .map(|(entity, _)| entity.clone())
            .collect();
        let mut scene = Self::from_entities(name, next_id, entities)?;
        let mut slots: BTreeMap<Option<EntityId>, BTreeMap<usize, EntityId>> = BTreeMap::new();

        for (entity, sibling_index) in ordered_entities {
            let group = slots.entry(entity.parent).or_default();
            if group.insert(sibling_index, entity.id).is_some() {
                return Err(SceneError::new(
                    operation,
                    "duplicate sibling index in a parent group",
                ));
            }
        }

        for (parent, members) in &mut scene.sibling_groups {
            let Some(group_slots) = slots.remove(parent) else {
                if members.is_empty() {
                    continue;
                }
                return Err(SceneError::new(operation, "missing sibling group indices"));
            };
            *members = contiguous_members(group_slots, operation)?;
        }
        if !slots.is_empty() {
            return Err(SceneError::new(operation, "unknown sibling group"));
        }

        scene.validate_for(operation)?;
        Ok(scene)
    }

    pub(super) fn move_sibling_group(
        &mut self,
        id: EntityId,
        old_parent: Option<EntityId>,
        new_parent: Option<EntityId>,
        operation: &'static str,
    ) -> Result<(), SceneError> {
        if old_parent == new_parent {
            return Ok(());
        }

        let source = self
            .sibling_groups
            .get(&old_parent)
            .ok_or_else(|| SceneError::new(operation, "source sibling group is missing"))?;
        let source_index = source
            .iter()
            .position(|member| *member == id)
            .ok_or_else(|| {
                SceneError::new(operation, "entity is missing from its sibling group")
            })?;
        let destination = self
            .sibling_groups
            .get(&new_parent)
            .ok_or_else(|| SceneError::new(operation, "destination sibling group is missing"))?;
        if destination.contains(&id) {
            return Err(SceneError::new(
                operation,
                "entity already exists in its destination sibling group",
            ));
        }

        self.sibling_groups
            .get_mut(&old_parent)
            .expect("source group was validated above")
            .remove(source_index);
        let destination = self
            .sibling_groups
            .get_mut(&new_parent)
            .expect("destination group was validated above");
        destination.push(id);
        Ok(())
    }

    /// Move an entity within its existing root or direct-child group.
    pub fn set_sibling_index(
        &mut self,
        id: EntityId,
        sibling_index: usize,
    ) -> Result<bool, SceneError> {
        let operation = "set sibling index";
        let entity = self
            .entities
            .get(&id)
            .ok_or_else(|| SceneError::new(operation, format!("entity {} does not exist", id.0)))?;
        let parent = entity.parent;
        let current_index = self.sibling_index(id).ok_or_else(|| {
            SceneError::new(operation, "entity is missing from its sibling group")
        })?;
        let group_length = self
            .sibling_groups
            .get(&parent)
            .map(Vec::len)
            .ok_or_else(|| SceneError::new(operation, "sibling group is missing"))?;

        if sibling_index >= group_length {
            return Err(SceneError::new(operation, "sibling index is out of range"));
        }
        if sibling_index == current_index {
            return Ok(false);
        }

        let group = self
            .sibling_groups
            .get_mut(&parent)
            .expect("sibling group was validated above");
        let moved = group.remove(current_index);
        group.insert(sibling_index, moved);
        self.hierarchy_revision = HierarchyRevision::fresh();
        Ok(true)
    }
}

fn contiguous_members(
    slots: BTreeMap<usize, EntityId>,
    operation: &'static str,
) -> Result<Vec<EntityId>, SceneError> {
    let mut members = Vec::with_capacity(slots.len());
    for (expected_index, (actual_index, id)) in slots.into_iter().enumerate() {
        if actual_index != expected_index {
            return Err(SceneError::new(
                operation,
                "sibling indices must form a contiguous zero-based group",
            ));
        }
        members.push(id);
    }
    Ok(members)
}
