//! Ordered scene, entity, and iterative forest validation.

use std::collections::{BTreeMap, BTreeSet};

use super::super::{MAX_ENTITIES, MAX_HIERARCHY_DEPTH, SceneError};
use super::{Scene, validate_name};

impl Scene {
    pub(super) fn validate_for(&self, operation: &'static str) -> Result<(), SceneError> {
        self.validate_topology(operation)?;
        self.validate_sibling_groups(operation)
    }

    pub(super) fn validate_topology(&self, operation: &'static str) -> Result<(), SceneError> {
        validate_name(&self.name, operation)?;
        if self.entities.len() > MAX_ENTITIES {
            return Err(SceneError::new(operation, "entity limit exceeded"));
        }
        if self.next_id == 0 {
            return Err(SceneError::new(
                operation,
                "allocation counter must be positive",
            ));
        }
        self.validate_entities(operation)?;
        self.validate_forest(operation)
    }

    fn validate_sibling_groups(&self, operation: &'static str) -> Result<(), SceneError> {
        let mut expected_parents = BTreeSet::from([None]);
        expected_parents.extend(self.entities.keys().copied().map(Some));
        if self.sibling_groups.keys().copied().collect::<BTreeSet<_>>() != expected_parents {
            return Err(SceneError::new(
                operation,
                "sibling groups do not match the scene entities",
            ));
        }

        let mut members = BTreeSet::new();
        for (parent, siblings) in &self.sibling_groups {
            for sibling in siblings {
                let entity = self.entities.get(sibling).ok_or_else(|| {
                    SceneError::new(operation, "sibling group contains a missing entity")
                })?;
                if entity.parent != *parent || !members.insert(*sibling) {
                    return Err(SceneError::new(
                        operation,
                        "sibling group membership is inconsistent",
                    ));
                }
            }
        }
        if members.len() != self.entities.len() {
            return Err(SceneError::new(
                operation,
                "an entity is missing from its sibling group",
            ));
        }
        Ok(())
    }

    fn validate_entities(&self, operation: &'static str) -> Result<(), SceneError> {
        for (id, entity) in &self.entities {
            if id.0 == 0 || entity.id != *id {
                return Err(SceneError::new(operation, "invalid entity identity"));
            }
            if id.0 >= self.next_id {
                return Err(SceneError::new(
                    operation,
                    "allocation counter could reuse an entity identity",
                ));
            }
            validate_name(&entity.name, operation)?;
            if !entity
                .transform
                .translation
                .iter()
                .chain(&entity.transform.rotation_deg)
                .chain(&entity.transform.scale)
                .all(|value| value.is_finite())
            {
                return Err(SceneError::new(
                    operation,
                    format!("entity {} has a nonfinite transform", id.0),
                ));
            }
            if let Some(parent) = entity.parent
                && !self.entities.contains_key(&parent)
            {
                return Err(SceneError::new(
                    operation,
                    format!("entity {} has missing parent {}", id.0, parent.0),
                ));
            }
        }
        Ok(())
    }

    fn validate_forest(&self, operation: &'static str) -> Result<(), SceneError> {
        // Memoized, iterative ancestry walks bound both stack use and work.
        let mut depths = BTreeMap::new();
        for id in self.entities.keys() {
            let mut path = Vec::new();
            let mut cursor = Some(*id);
            let mut depth = 0;
            while let Some(current) = cursor {
                if let Some(known_depth) = depths.get(&current) {
                    depth = *known_depth;
                    break;
                }
                if path.contains(&current) {
                    return Err(SceneError::new(
                        operation,
                        format!("hierarchy cycle at entity {}", current.0),
                    ));
                }
                path.push(current);
                if path.len() > MAX_HIERARCHY_DEPTH {
                    return Err(SceneError::new(operation, "hierarchy depth limit exceeded"));
                }
                cursor = self.entities[&current].parent;
            }
            if depth + path.len() > MAX_HIERARCHY_DEPTH {
                return Err(SceneError::new(operation, "hierarchy depth limit exceeded"));
            }
            for current in path.into_iter().rev() {
                depth += 1;
                depths.insert(current, depth);
            }
        }
        Ok(())
    }
}
