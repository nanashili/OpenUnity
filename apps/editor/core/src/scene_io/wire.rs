use serde::{Deserialize, Serialize};

use crate::scene::{Entity, Scene};
use crate::write_guard::MAX_EXISTING_BYTES;

use super::SceneIoError;
use super::depth::check_json_depth;

const FORMAT: &str = "openunity.scene";
const VERSION: u32 = 1;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Document {
    format: String,
    version: u32,
    name: String,
    next_id: u64,
    entities: Vec<Entity>,
}

#[derive(Serialize)]
struct DocumentRef<'a> {
    format: &'static str,
    version: u32,
    name: &'a str,
    next_id: u64,
    entities: Vec<&'a Entity>,
}

/// Encode a validated scene in the independent v1 wire format.
pub fn encode(scene: &Scene) -> Result<Vec<u8>, SceneIoError> {
    scene
        .validate()
        .map_err(|cause| SceneIoError::new("encode scene", cause))?;
    let document = DocumentRef {
        format: FORMAT,
        version: VERSION,
        name: scene.name(),
        next_id: scene.next_id(),
        entities: scene.entities().collect(),
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

/// Decode without repairing invalid identities, counters, or hierarchy links.
pub fn decode(bytes: &[u8]) -> Result<Scene, SceneIoError> {
    if bytes.len() > MAX_EXISTING_BYTES {
        return Err(SceneIoError::new(
            "decode scene",
            "document exceeds the 16 MiB limit",
        ));
    }
    check_json_depth(bytes)?;
    let document: Document =
        serde_json::from_slice(bytes).map_err(|cause| SceneIoError::new("decode scene", cause))?;
    if document.format != FORMAT {
        return Err(SceneIoError::new(
            "decode scene",
            "unsupported document format",
        ));
    }
    if document.version != VERSION {
        return Err(SceneIoError::new(
            "decode scene",
            "unsupported scene version",
        ));
    }
    Scene::from_entities(document.name, document.next_id, document.entities)
        .map_err(|cause| SceneIoError::new("decode scene", cause))
}
