//! Independently specified scene authoring model (INFRA-EDITOR-001).

use std::collections::BTreeMap;
use std::fmt;

use serde::{Deserialize, Serialize};

/// Maximum UTF-8 bytes in a scene or entity name.
pub const MAX_NAME_BYTES: usize = 1_024;
/// Maximum accepted entities in one document.
pub const MAX_ENTITIES: usize = 100_000;
/// Maximum hierarchy depth, counting a root as depth one.
pub const MAX_HIERARCHY_DEPTH: usize = 256;

/// Stable positive identity scoped to one open scene document.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(transparent)]
pub struct EntityId(pub u64);

/// Independently authored primitive representations.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum Primitive {
    /// An entity without geometry.
    Empty,
    /// A unit cube.
    Cube,
    /// An authoring camera representation.
    Camera,
}

/// Local transform; negative and zero scale are valid.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Transform {
    /// Local translation.
    pub translation: [f32; 3],
    /// Local rotation in degrees.
    pub rotation_deg: [f32; 3],
    /// Local scale.
    pub scale: [f32; 3],
}

impl Default for Transform {
    fn default() -> Self {
        Self {
            translation: [0.0; 3],
            rotation_deg: [0.0; 3],
            scale: [1.0; 3],
        }
    }
}

/// An entity value; changes enter a scene only through validated operations.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Entity {
    /// Stable document identity.
    pub id: EntityId,
    /// Nonempty name, bounded by [`MAX_NAME_BYTES`].
    pub name: String,
    /// Optional parent in the same scene.
    #[serde(deserialize_with = "deserialize_parent")]
    pub parent: Option<EntityId>,
    /// Whether the entity is active.
    pub active: bool,
    /// Local transform.
    pub transform: Transform,
    /// Authoring representation.
    pub primitive: Primitive,
}

fn deserialize_parent<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<Option<EntityId>, D::Error> {
    Option::<EntityId>::deserialize(deserializer)
}

/// An authoring failure with the operation and a user-facing explanation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SceneError {
    operation: &'static str,
    detail: String,
}

impl SceneError {
    fn new(operation: &'static str, detail: impl Into<String>) -> Self {
        Self {
            operation,
            detail: detail.into(),
        }
    }

    /// The operation that failed.
    pub fn operation(&self) -> &'static str {
        self.operation
    }

    /// The reason the operation failed.
    pub fn detail(&self) -> &str {
        &self.detail
    }
}

impl fmt::Display for SceneError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}: {}", self.operation, self.detail)
    }
}

impl std::error::Error for SceneError {}

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
        Ok(removed)
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

    fn validate_for(&self, operation: &'static str) -> Result<(), SceneError> {
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

fn validate_name(name: &str, operation: &'static str) -> Result<(), SceneError> {
    if name.is_empty() {
        return Err(SceneError::new(operation, "name must not be empty"));
    }
    if name.len() > MAX_NAME_BYTES {
        return Err(SceneError::new(operation, "name exceeds 1024 UTF-8 bytes"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture(id: u64, parent: Option<u64>) -> Entity {
        Entity {
            id: EntityId(id),
            name: format!("Entity {id}"),
            parent: parent.map(EntityId),
            active: true,
            transform: Transform::default(),
            primitive: Primitive::Empty,
        }
    }

    #[test]
    fn subtree_removal_and_history_restore_never_reuse_allocated_ids() {
        let mut scene = Scene::new("Scene").unwrap();
        let root = scene.create("Root", Primitive::Cube, None).unwrap();
        let child = scene.create("Child", Primitive::Empty, Some(root)).unwrap();
        let snapshot = scene.clone();
        let unrelated = scene.create("Camera", Primitive::Camera, None).unwrap();
        assert_eq!(scene.remove_subtree(root).unwrap(), vec![root, child]);
        assert!(scene.get(unrelated).is_some());
        assert!(scene.get(child).is_none());
        let high_water = scene.next_id();
        scene = snapshot;
        scene.preserve_high_water(high_water).unwrap();
        assert!(scene.get(child).is_some());
        assert_eq!(
            scene.create("New", Primitive::Empty, None).unwrap(),
            EntityId(4)
        );
        assert_eq!(
            scene.entities().map(|entity| entity.id).collect::<Vec<_>>(),
            vec![EntityId(1), EntityId(2), EntityId(4)]
        );
    }

    #[test]
    fn invalid_entity_edits_are_atomic() {
        let mut scene = Scene::new("Scene").unwrap();
        let root = scene.create("Root", Primitive::Empty, None).unwrap();
        let child = scene.create("Child", Primitive::Empty, Some(root)).unwrap();
        let before = scene.clone();
        let invalid: [fn(&mut Entity); 7] = [
            |entity| entity.name.clear(),
            |entity| entity.name = "é".repeat(513),
            |entity| entity.transform.translation[0] = f32::NAN,
            |entity| entity.transform.rotation_deg[2] = f32::INFINITY,
            |entity| entity.transform.scale[1] = f32::NEG_INFINITY,
            |entity| entity.parent = Some(EntityId(999)),
            |entity| entity.id = EntityId(999),
        ];
        for edit in invalid {
            assert!(scene.edit(root, edit).is_err());
            assert_eq!(scene, before);
        }
        assert!(
            scene
                .edit(root, |entity| entity.parent = Some(child))
                .is_err()
        );
        assert!(
            scene
                .edit(root, |entity| entity.parent = Some(root))
                .is_err()
        );
        assert!(scene.edit(EntityId(999), |_| {}).is_err());
        assert_eq!(scene, before);
        scene
            .edit(child, |entity| {
                entity.active = false;
                entity.transform.scale = [-1.0, 0.0, 2.0];
                entity.name = "é".repeat(512);
            })
            .unwrap();
        assert!(!scene.get(child).unwrap().active);
        assert_eq!(scene.get(child).unwrap().transform.scale, [-1.0, 0.0, 2.0]);
    }

    #[test]
    fn reconstruction_rejects_invalid_ids_parents_cycles_and_counters() {
        let cases = [
            (2, vec![fixture(0, None)]),
            (2, vec![fixture(1, None), fixture(1, None)]),
            (1, vec![fixture(1, None)]),
            (0, vec![]),
            (2, vec![fixture(1, Some(2))]),
            (2, vec![fixture(1, Some(1))]),
            (3, vec![fixture(1, Some(2)), fixture(2, Some(1))]),
        ];
        for (counter, entities) in cases {
            assert!(Scene::from_entities("Scene", counter, entities).is_err());
        }
        let scene =
            Scene::from_entities("Scene", 3, vec![fixture(2, None), fixture(1, Some(2))]).unwrap();
        assert_eq!(
            scene.entities().map(|entity| entity.id).collect::<Vec<_>>(),
            vec![EntityId(1), EntityId(2)]
        );
    }

    #[test]
    fn hierarchy_depth_limit_applies_to_creation_loading_and_reparenting() {
        let chain: Vec<_> = (1..=256)
            .map(|id| fixture(id, (id > 1).then_some(id - 1)))
            .collect();
        let mut scene = Scene::from_entities("Scene", 257, chain.clone()).unwrap();
        let before = scene.clone();
        assert!(
            scene
                .create("Too deep", Primitive::Empty, Some(EntityId(256)))
                .is_err()
        );
        assert_eq!(scene, before);
        let extra = scene.create("Root", Primitive::Empty, None).unwrap();
        let before = scene.clone();
        assert!(
            scene
                .edit(EntityId(1), |entity| entity.parent = Some(extra))
                .is_err()
        );
        assert_eq!(scene, before);
        let mut too_deep = chain;
        too_deep.push(fixture(257, Some(256)));
        assert!(Scene::from_entities("Scene", 258, too_deep).is_err());
    }

    #[test]
    fn names_entity_limit_and_identity_exhaustion_preserve_state() {
        assert!(Scene::new("").is_err());
        assert!(Scene::new("é".repeat(513)).is_err());
        let mut scene = Scene::new("é".repeat(512)).unwrap();
        let before = scene.clone();
        assert!(scene.set_name("").is_err());
        assert!(scene.create("", Primitive::Empty, None).is_err());
        assert!(
            scene
                .create("Missing", Primitive::Empty, Some(EntityId(1)))
                .is_err()
        );
        assert!(scene.remove_subtree(EntityId(1)).is_err());
        assert!(scene.preserve_high_water(0).is_err());
        assert_eq!(scene, before);
        scene.preserve_high_water(u64::MAX - 1).unwrap();
        assert_eq!(
            scene.create("Last", Primitive::Empty, None).unwrap(),
            EntityId(u64::MAX - 1)
        );
        let before = scene.clone();
        assert!(scene.create("Exhausted", Primitive::Empty, None).is_err());
        assert_eq!(scene, before);
        let entities: Vec<_> = (1..=100_000).map(|id| fixture(id, None)).collect();
        let mut full = Scene::from_entities("Full", 100_001, entities.clone()).unwrap();
        assert!(full.create("Extra", Primitive::Empty, None).is_err());
        assert_eq!(full.entities().len(), 100_000);
        assert_eq!(full.next_id(), 100_001);
        let mut overfull = entities;
        overfull.push(fixture(100_001, None));
        assert!(Scene::from_entities("Overfull", 100_002, overfull).is_err());
    }

    #[test]
    fn entity_wire_types_reject_unknown_and_duplicate_fields() {
        for json in [
            r#"{"translation":[0,0,0],"rotation_deg":[0,0,0],"scale":[1,1,1],"extra":true}"#,
            r#"{"translation":[0,0,0],"translation":[1,1,1],"rotation_deg":[0,0,0],"scale":[1,1,1]}"#,
        ] {
            assert!(serde_json::from_str::<Transform>(json).is_err());
        }
        let mut value = serde_json::to_value(fixture(1, None)).unwrap();
        value["unexpected"] = serde_json::Value::Bool(true);
        assert!(serde_json::from_value::<Entity>(value).is_err());
    }
}
