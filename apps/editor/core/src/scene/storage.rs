//! Validated scene ownership, reconstruction, queries, and allocation state.

use std::collections::BTreeMap;

use super::{Entity, EntityId, MAX_ENTITIES, MAX_NAME_BYTES, SceneError};

mod edits;
mod validation;

#[cfg(test)]
mod tests;

/// Validated scene storage with monotonically increasing allocation state.
///
/// Generic history must call [`Self::preserve_high_water`] with the live
/// document's [`Self::next_id`] before committing a restored snapshot. The
/// serialized document wrapper belongs to scene persistence, not this model.
#[derive(Clone, Debug, PartialEq)]
pub struct Scene {
    name: String,
    entities: BTreeMap<EntityId, Entity>,
    next_id: u64,
}

impl Scene {
    /// Create an empty named document.
    pub fn new(name: impl Into<String>) -> Result<Self, SceneError> {
        let name = name.into();
        validate_name(&name, "create scene")?;
        Ok(Self {
            name,
            entities: BTreeMap::new(),
            next_id: 1,
        })
    }

    /// Reconstruct a document, rejecting duplicates before indexing its array.
    /// The persisted allocation counter must exceed every stored identity.
    pub fn from_entities(
        name: impl Into<String>,
        next_id: u64,
        entities: Vec<Entity>,
    ) -> Result<Self, SceneError> {
        let operation = "load scene";
        if entities.len() > MAX_ENTITIES {
            return Err(SceneError::new(operation, "entity limit exceeded"));
        }
        let mut scene = Self {
            name: name.into(),
            next_id,
            entities: BTreeMap::new(),
        };
        for entity in entities {
            let id = entity.id;
            if scene.entities.insert(id, entity).is_some() {
                return Err(SceneError::new(
                    operation,
                    format!("duplicate entity identity {}", id.0),
                ));
            }
        }
        scene.validate_for(operation)?;
        Ok(scene)
    }

    /// Document name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Set the document name, preserving state on error.
    pub fn set_name(&mut self, name: impl Into<String>) -> Result<(), SceneError> {
        let name = name.into();
        validate_name(&name, "rename scene")?;
        self.name = name;
        Ok(())
    }

    /// Entities in ascending identity order.
    pub fn entities(&self) -> impl ExactSizeIterator<Item = &Entity> + DoubleEndedIterator {
        self.entities.values()
    }

    /// Borrow one live entity.
    pub fn get(&self, id: EntityId) -> Option<&Entity> {
        self.entities.get(&id)
    }

    /// Validate names, identities, finite transforms and the entire forest.
    pub fn validate(&self) -> Result<(), SceneError> {
        self.validate_for("validate scene")
    }

    /// Next identity counter, also the session allocation high-water mark.
    pub fn next_id(&self) -> u64 {
        self.next_id
    }

    /// Preserve allocation state while restoring an earlier history snapshot.
    /// `minimum` is a next-identity counter, not the last allocated identity.
    pub fn preserve_high_water(&mut self, minimum: u64) -> Result<(), SceneError> {
        if minimum == 0 {
            return Err(SceneError::new(
                "preserve scene identities",
                "allocation counter must be positive",
            ));
        }
        self.next_id = self.next_id.max(minimum);
        Ok(())
    }
}

fn validate_name(name: &str, operation: &'static str) -> Result<(), SceneError> {
    if name.is_empty() {
        return Err(SceneError::new(operation, "name must not be empty"));
    }
    if name.len() > MAX_NAME_BYTES {
        return Err(SceneError::new(operation, "name exceeds 1024 UTF-8 bytes"));
    }
    Ok(())
}
