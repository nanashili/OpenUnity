//! Validated entity edits, creation, and subtree removal.

use std::collections::BTreeMap;

use crate::hierarchy_revision::HierarchyRevision;

use super::super::{
    Entity, EntityId, MAX_ENTITIES, MAX_HIERARCHY_DEPTH, Primitive, SceneError, Transform,
};
use super::{Scene, validate_name};

impl Scene {
    /// Edit one entity transactionally. Identity changes are forbidden.
    ///
    /// Parent, name and transform validation runs before replacing live state.
    /// A mutable entity reference is deliberately never exposed by this model.
    pub fn edit(&mut self, id: EntityId, edit: impl FnOnce(&mut Entity)) -> Result<(), SceneError> {
        let operation = "edit entity";
        if !self.entities.contains_key(&id) {
            return Err(SceneError::new(
                operation,
                format!("entity {} does not exist", id.0),
            ));
        }
        let mut candidate = self.clone();
        // Existence was checked above; the clone contains the same identities.
        let entity = candidate
            .entities
            .get_mut(&id)
            .expect("checked entity exists");
        edit(entity);
        if entity.id != id {
            return Err(SceneError::new(
                operation,
                "entity identity cannot be changed",
            ));
        }
        candidate.validate_for(operation)?;
        candidate.hierarchy_revision = HierarchyRevision::fresh();
        *self = candidate;
        Ok(())
    }

    /// Create an active entity with a default local transform.
    pub fn create(
        &mut self,
        name: impl Into<String>,
        primitive: Primitive,
        parent: Option<EntityId>,
    ) -> Result<EntityId, SceneError> {
        let operation = "create entity";
        let name = name.into();
        validate_name(&name, operation)?;
        if self.entities.len() >= MAX_ENTITIES {
            return Err(SceneError::new(operation, "entity limit exceeded"));
        }
        let mut depth = 1;
        let mut ancestor = parent;
        while let Some(id) = ancestor {
            let entity = self.entities.get(&id).ok_or_else(|| {
                SceneError::new(operation, format!("parent {} does not exist", id.0))
            })?;
            depth += 1;
            if depth > MAX_HIERARCHY_DEPTH {
                return Err(SceneError::new(operation, "hierarchy depth limit exceeded"));
            }
            ancestor = entity.parent;
        }
        // u64::MAX is the exhausted next-counter sentinel; it cannot be
        // allocated because its successor cannot be persisted in this format.
        let next_id = self
            .next_id
            .checked_add(1)
            .ok_or_else(|| SceneError::new(operation, "entity identity space exhausted"))?;
        let id = EntityId(self.next_id);
        self.entities.insert(
            id,
            Entity {
                id,
                name,
                parent,
                active: true,
                transform: Transform::default(),
                primitive,
            },
        );
        self.next_id = next_id;
        self.hierarchy_revision = HierarchyRevision::fresh();
        Ok(id)
    }

    /// Remove an entity and all descendants, returning IDs in ascending order.
    pub fn remove_subtree(&mut self, id: EntityId) -> Result<Vec<EntityId>, SceneError> {
        let operation = "remove subtree";
        if !self.entities.contains_key(&id) {
            return Err(SceneError::new(
                operation,
                format!("entity {} does not exist", id.0),
            ));
        }
        let mut children: BTreeMap<EntityId, Vec<EntityId>> = BTreeMap::new();
        for entity in self.entities.values() {
            if let Some(parent) = entity.parent {
                children.entry(parent).or_default().push(entity.id);
            }
        }
        let mut pending = vec![id];
        let mut removed = Vec::new();
        while let Some(current) = pending.pop() {
            if let Some(descendants) = children.get(&current) {
                pending.extend(descendants);
            }
            removed.push(current);
        }
        removed.sort_unstable();
        for id in &removed {
            self.entities.remove(id);
        }
        self.hierarchy_revision = HierarchyRevision::fresh();
        Ok(removed)
    }
}
