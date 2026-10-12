//! Independent serialized entity identities, primitives, and local transforms.

use serde::{Deserialize, Serialize};

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
    /// Nonempty name, bounded by [`super::MAX_NAME_BYTES`].
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
