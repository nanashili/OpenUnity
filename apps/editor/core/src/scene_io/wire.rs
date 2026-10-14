use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use crate::scene::{Entity, EntityId, Primitive, Scene, Transform};
use crate::write_guard::MAX_EXISTING_BYTES;

use super::SceneIoError;
use super::depth::check_json_depth;

const FORMAT: &str = "openunity.scene";
const VERSION_V1: u32 = 1;
const VERSION_V2: u32 = 2;

#[derive(Deserialize)]
struct DocumentHeader {
    format: String,
    version: u32,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct DocumentV1 {
    format: String,
    version: u32,
    name: String,
    next_id: u64,
    entities: Vec<Entity>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct DocumentV2 {
    format: String,
    version: u32,
    name: String,
    next_id: u64,
    entities: Vec<OrderedEntity>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct OrderedEntity {
    id: EntityId,
    name: String,
    #[serde(deserialize_with = "deserialize_parent")]
    parent: Option<EntityId>,
    active: bool,
    transform: Transform,
    primitive: Primitive,
    sibling_index: usize,
}

#[derive(Serialize)]
struct DocumentRef<'a> {
    format: &'static str,
    version: u32,
    name: &'a str,
    next_id: u64,
    entities: Vec<OrderedEntityRef<'a>>,
}

#[derive(Serialize)]
struct OrderedEntityRef<'a> {
    id: EntityId,
    name: &'a str,
    parent: Option<EntityId>,
    active: bool,
    transform: Transform,
    primitive: Primitive,
    sibling_index: usize,
}

/// Encode a validated scene in the independent v2 wire format.
pub fn encode(scene: &Scene) -> Result<Vec<u8>, SceneIoError> {
    scene
        .validate()
        .map_err(|cause| SceneIoError::new("encode scene", cause))?;

    let sibling_indices = sibling_indices(scene);
    let entities = scene
        .entities()
        .map(|entity| OrderedEntityRef {
            id: entity.id,
            name: &entity.name,
            parent: entity.parent,
            active: entity.active,
            transform: entity.transform,
            primitive: entity.primitive,
            sibling_index: sibling_indices
                .get(&entity.id)
                .copied()
                .expect("validated entity belongs to one sibling group"),
        })
        .collect();
    let document = DocumentRef {
        format: FORMAT,
        version: VERSION_V2,
        name: scene.name(),
        next_id: scene.next_id(),
        entities,
    };
    let bytes =
        serde_json::to_vec(&document).map_err(|cause| SceneIoError::new("encode scene", cause))?;
    if bytes.len() > MAX_EXISTING_BYTES {
        return Err(SceneIoError::new(
            "encode scene",
            "document exceeds the 16 MiB limit",
        ));
    }
    Ok(bytes)
}

fn sibling_indices(scene: &Scene) -> BTreeMap<EntityId, usize> {
    let parents = std::iter::once(None).chain(scene.entities().map(|entity| Some(entity.id)));
    let mut indices = BTreeMap::new();

    for parent in parents {
        if let Some(siblings) = scene.sibling_group(parent) {
            for (index, sibling) in siblings.iter().copied().enumerate() {
                indices.insert(sibling, index);
            }
        }
    }

    indices
}

/// Decode v1 files using their legacy ID order and v2 files using explicit groups.
pub fn decode(bytes: &[u8]) -> Result<Scene, SceneIoError> {
    if bytes.len() > MAX_EXISTING_BYTES {
        return Err(SceneIoError::new(
            "decode scene",
            "document exceeds the 16 MiB limit",
        ));
    }
    check_json_depth(bytes)?;

    let header: DocumentHeader =
        serde_json::from_slice(bytes).map_err(|cause| SceneIoError::new("decode scene", cause))?;
    if header.format != FORMAT {
        return Err(SceneIoError::new(
            "decode scene",
            "unsupported document format",
        ));
    }
    match header.version {
        VERSION_V1 => decode_v1(bytes),
        VERSION_V2 => decode_v2(bytes),
        _ => Err(SceneIoError::new(
            "decode scene",
            "unsupported scene version",
        )),
    }
}

fn decode_v1(bytes: &[u8]) -> Result<Scene, SceneIoError> {
    let document: DocumentV1 =
        serde_json::from_slice(bytes).map_err(|cause| SceneIoError::new("decode scene", cause))?;
    if document.format != FORMAT || document.version != VERSION_V1 {
        return Err(SceneIoError::new("decode scene", "invalid v1 scene header"));
    }
    Scene::from_entities(document.name, document.next_id, document.entities)
        .map_err(|cause| SceneIoError::new("decode scene", cause))
}

fn decode_v2(bytes: &[u8]) -> Result<Scene, SceneIoError> {
    let document: DocumentV2 =
        serde_json::from_slice(bytes).map_err(|cause| SceneIoError::new("decode scene", cause))?;
    if document.format != FORMAT || document.version != VERSION_V2 {
        return Err(SceneIoError::new("decode scene", "invalid v2 scene header"));
    }
    let ordered_entities = document
        .entities
        .into_iter()
        .map(|entity| {
            (
                Entity {
                    id: entity.id,
                    name: entity.name,
                    parent: entity.parent,
                    active: entity.active,
                    transform: entity.transform,
                    primitive: entity.primitive,
                },
                entity.sibling_index,
            )
        })
        .collect();
    Scene::from_ordered_entities(document.name, document.next_id, ordered_entities)
        .map_err(|cause| SceneIoError::new("decode scene", cause))
}

fn deserialize_parent<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<Option<EntityId>, D::Error> {
    Option::<EntityId>::deserialize(deserializer)
}
