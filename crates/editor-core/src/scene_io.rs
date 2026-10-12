//! Bounded persistence for independent OpenUnity scene documents.

use std::fmt;
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::scene::{Entity, Scene};
use crate::write_guard::{MAX_EXISTING_BYTES, WritePolicy};

const FORMAT: &str = "openunity.scene";
const VERSION: u32 = 1;
const MAX_JSON_DEPTH: usize = 32;

/// A rejected scene document or failed scene file operation.
#[derive(Debug)]
pub struct SceneIoError {
    /// The operation that failed.
    pub operation: &'static str,
    /// The file involved, when the operation used the filesystem.
    pub path: Option<PathBuf>,
    /// The reason for the failure.
    pub message: String,
}

impl SceneIoError {
    fn new(operation: &'static str, message: impl fmt::Display) -> Self {
        Self {
            operation,
            path: None,
            message: message.to_string(),
        }
    }

    fn at(mut self, operation: &'static str, path: &Path) -> Self {
        self.operation = operation;
        self.path = Some(path.to_path_buf());
        self
    }
}

impl fmt::Display for SceneIoError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}", self.operation)?;
        if let Some(path) = &self.path {
            write!(formatter, " {}", path.display())?;
        }
        write!(formatter, ": {}", self.message)
    }
}

impl std::error::Error for SceneIoError {}

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

// Count containers before serde allocates document fields. Escapes apply to
// exactly the following byte; brackets and escaped quotes inside strings do
// not alter nesting. Serde subsequently validates all JSON grammar and UTF-8.
fn check_json_depth(bytes: &[u8]) -> Result<(), SceneIoError> {
    let mut depth = 0_usize;
    let mut in_string = false;
    let mut escaped = false;
    for &byte in bytes {
        if in_string {
            if escaped {
                escaped = false;
            } else if byte == b'\\' {
                escaped = true;
            } else if byte == b'"' {
                in_string = false;
            }
        } else {
            match byte {
                b'"' => in_string = true,
                b'{' | b'[' => {
                    depth += 1;
                    if depth > MAX_JSON_DEPTH {
                        return Err(SceneIoError::new(
                            "decode scene",
                            "JSON nesting exceeds 32 levels",
                        ));
                    }
                }
                b'}' | b']' => depth = depth.saturating_sub(1),
                _ => {}
            }
        }
    }
    Ok(())
}

fn check_extension(path: &Path, operation: &'static str) -> Result<(), SceneIoError> {
    if path
        .extension()
        .is_some_and(|extension| extension == "ouscene")
    {
        Ok(())
    } else {
        Err(SceneIoError::new(
            operation,
            "independent scenes require the .ouscene extension",
        )
        .at(operation, path))
    }
}

/// Read a bounded `.ouscene` document through the shared path guard.
pub fn load(path: impl AsRef<Path>, policy: &WritePolicy) -> Result<Scene, SceneIoError> {
    let path = path.as_ref();
    let operation = "load scene";
    check_extension(path, operation)?;
    policy
        .check_read(path)
        .map_err(|cause| SceneIoError::new(operation, cause).at(operation, path))?;
    let metadata = std::fs::metadata(path)
        .map_err(|cause| SceneIoError::new(operation, cause).at(operation, path))?;
    if !metadata.is_file() {
        return Err(
            SceneIoError::new(operation, "scene input must be a regular file").at(operation, path),
        );
    }
    if metadata.len() > MAX_EXISTING_BYTES as u64 {
        return Err(
            SceneIoError::new(operation, "document exceeds the 16 MiB limit").at(operation, path),
        );
    }
    let file = File::open(path)
        .map_err(|cause| SceneIoError::new(operation, cause).at(operation, path))?;
    let mut bytes = Vec::new();
    file.take(MAX_EXISTING_BYTES as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|cause| SceneIoError::new(operation, cause).at(operation, path))?;
    decode(&bytes).map_err(|cause| cause.at(operation, path))
}

/// Atomically save outside protected roots, replacing only a validated owned scene.
pub fn save(
    path: impl AsRef<Path>,
    scene: &Scene,
    policy: &WritePolicy,
) -> Result<(), SceneIoError> {
    let path = path.as_ref();
    let operation = "save scene";
    check_extension(path, operation)?;
    let bytes = encode(scene).map_err(|cause| cause.at(operation, path))?;
    policy
        .atomic_write(path, &bytes, |existing| {
            decode(existing)
                .map(|_| ())
                .map_err(|cause| cause.to_string())
        })
        .map_err(|cause| SceneIoError::new(operation, cause).at(operation, path))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scene::{EntityId, Primitive};
    use std::fs;
    use std::sync::atomic::{AtomicU64, Ordering};

    // Independently authored literal contract fixture, never reference output.
    const FIXTURE: &str = r#"{"format":"openunity.scene","version":1,"name":"Authored fixture","next_id":9,"entities":[{"id":2,"name":"Root","parent":null,"active":true,"transform":{"translation":[1,2,3],"rotation_deg":[0,90,0],"scale":[-1,0,2]},"primitive":"Cube"},{"id":7,"name":"Child","parent":2,"active":false,"transform":{"translation":[0,0,0],"rotation_deg":[0,0,0],"scale":[1,1,1]},"primitive":"Camera"}]}"#;

    static NEXT_DIRECTORY: AtomicU64 = AtomicU64::new(0);

    struct Directory(PathBuf);

    impl Directory {
        fn new() -> Self {
            let base = std::env::temp_dir().canonicalize().unwrap();
            let sequence = NEXT_DIRECTORY.fetch_add(1, Ordering::Relaxed);
            let path = base.join(format!(
                "openunity-scene-io-test-{}-{sequence}",
                std::process::id()
            ));
            fs::create_dir(&path).unwrap();
            Self(path)
        }
    }

    impl Drop for Directory {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn authored_fixture_roundtrips_and_preserves_allocation_counter() {
        let mut scene = decode(FIXTURE.as_bytes()).unwrap();
        assert_eq!(scene.name(), "Authored fixture");
        assert_eq!(scene.get(EntityId(7)).unwrap().parent, Some(EntityId(2)));
        assert_eq!(
            scene.get(EntityId(2)).unwrap().transform.scale,
            [-1.0, 0.0, 2.0]
        );
        assert_eq!(decode(&encode(&scene).unwrap()).unwrap(), scene);
        assert_eq!(
            scene.create("New", Primitive::Empty, None).unwrap(),
            EntityId(9)
        );
    }

    #[test]
    fn malformed_documents_are_rejected_without_id_repair() {
        let cases = [
            FIXTURE.replace("\"next_id\":9", "\"next_id\":7"),
            FIXTURE.replace("\"id\":7", "\"id\":2"),
            FIXTURE.replace("\"id\":2", "\"id\":0"),
            FIXTURE.replace("\"version\":1", "\"version\":2"),
            FIXTURE.replace("\"version\":1", "\"version\":1.0"),
            FIXTURE.replace("openunity.scene", "unrelated.scene"),
            FIXTURE.replace("\"format\":", "\"extra\":true,\"format\":"),
            FIXTURE.replace("\"version\":1", "\"version\":1,\"version\":1"),
            FIXTURE.replace("\"id\":7", "\"id\":7,\"id\":7"),
            FIXTURE.replace("\"id\":7", "\"extra\":true,\"id\":7"),
            FIXTURE.replace("\"scale\":[1,1,1]", "\"scale\":[1,1,1],\"extra\":0"),
            FIXTURE.replace("\"scale\":[1,1,1]", "\"scale\":[1,1,1],\"scale\":[1,1,1]"),
            FIXTURE.replace("\"parent\":2", "\"parent\":7"),
            FIXTURE.replace("\"parent\":2", "\"parent\":99"),
            FIXTURE.replace("\"parent\":2,", ""),
            FIXTURE.replace("\"name\":\"Child\"", "\"name\":\"\""),
            FIXTURE.replace("\"scale\":[1,1,1]", "\"scale\":[1e100,1,1]"),
            format!("{FIXTURE} trailing"),
            "{".into(),
        ];
        for bytes in cases {
            assert!(decode(bytes.as_bytes()).is_err(), "accepted {bytes}");
        }
        assert!(decode(br#"{"format":"openunity.scene","version":1,"name":"Scene","next_id":1,"entities":{}}"#).is_err());
    }

    #[test]
    fn parser_bounds_bytes_and_depth_without_counting_string_brackets() {
        let oversized = vec![b' '; MAX_EXISTING_BYTES + 1];
        assert!(decode(&oversized).unwrap_err().message.contains("16 MiB"));
        let at_limit = format!("{}0{}", "[".repeat(32), "]".repeat(32));
        assert!(
            !decode(at_limit.as_bytes())
                .unwrap_err()
                .message
                .contains("nesting")
        );
        let too_deep = format!("{}0{}", "[".repeat(33), "]".repeat(33));
        assert!(
            decode(too_deep.as_bytes())
                .unwrap_err()
                .message
                .contains("nesting")
        );
        let mut scene = Scene::new(format!(r#"Quoted \" {} \\"#, "[{]".repeat(200))).unwrap();
        scene.create("Entity", Primitive::Empty, None).unwrap();
        assert_eq!(decode(&encode(&scene).unwrap()).unwrap(), scene);
    }

    #[test]
    fn oversized_encoded_scenes_never_replace_a_document() {
        let entities = (1..=16_384)
            .map(|id| Entity {
                id: EntityId(id),
                name: "n".repeat(crate::scene::MAX_NAME_BYTES),
                parent: None,
                active: true,
                transform: crate::scene::Transform::default(),
                primitive: Primitive::Empty,
            })
            .collect();
        let scene = Scene::from_entities("Large", 16_385, entities).unwrap();
        assert!(encode(&scene).unwrap_err().message.contains("16 MiB"));
        let directory = Directory::new();
        let path = directory.0.join("scene.ouscene");
        fs::write(&path, FIXTURE).unwrap();
        let policy = WritePolicy::new([]).unwrap();
        assert!(save(&path, &scene, &policy).is_err());
        assert_eq!(fs::read(&path).unwrap(), FIXTURE.as_bytes());
    }

    #[test]
    fn file_roundtrip_and_owned_overwrite_preserve_unrelated_files() {
        let directory = Directory::new();
        let policy = WritePolicy::new([]).unwrap();
        let path = directory.0.join("scene.ouscene");
        let mut scene = decode(FIXTURE.as_bytes()).unwrap();
        save(&path, &scene, &policy).unwrap();
        assert_eq!(load(&path, &policy).unwrap(), scene);
        scene.set_name("Saved again").unwrap();
        save(&path, &scene, &policy).unwrap();
        assert_eq!(load(&path, &policy).unwrap(), scene);
        for unrelated in [
            b"unrelated document".as_slice(),
            br#"{"format":"openunity.scene","version":1}"#.as_slice(),
        ] {
            fs::write(&path, unrelated).unwrap();
            assert!(save(&path, &scene, &policy).is_err());
            assert_eq!(fs::read(&path).unwrap(), unrelated);
        }
        for extension in ["unity", "json", "OUSCENE"] {
            let forbidden = directory.0.join(format!("scene.{extension}"));
            fs::write(&forbidden, FIXTURE).unwrap();
            assert!(load(&forbidden, &policy).is_err());
            assert!(save(&forbidden, &scene, &policy).is_err());
            assert_eq!(fs::read(&forbidden).unwrap(), FIXTURE.as_bytes());
        }
        assert!(load(directory.0.join("missing.ouscene"), &policy).is_err());
        let oversized = directory.0.join("oversized.ouscene");
        File::create(&oversized)
            .unwrap()
            .set_len(MAX_EXISTING_BYTES as u64 + 1)
            .unwrap();
        assert!(
            load(&oversized, &policy)
                .unwrap_err()
                .message
                .contains("16 MiB")
        );
    }

    #[test]
    fn protected_roots_reject_new_and_existing_scene_writes() {
        let directory = Directory::new();
        let protected = directory.0.join("source");
        fs::create_dir(&protected).unwrap();
        let path = protected.join("existing.ouscene");
        fs::write(&path, FIXTURE).unwrap();
        let policy = WritePolicy::new([protected.clone()]).unwrap();
        let scene = decode(FIXTURE.as_bytes()).unwrap();
        assert_eq!(load(&path, &policy).unwrap(), scene);
        assert!(save(&path, &scene, &policy).is_err());
        assert_eq!(fs::read(&path).unwrap(), FIXTURE.as_bytes());
        let missing = protected.join("new.ouscene");
        assert!(save(&missing, &scene, &policy).is_err());
        assert!(!missing.exists());
    }

    #[cfg(unix)]
    #[test]
    fn symlink_reads_and_writes_are_rejected_at_parent_and_final_components() {
        use std::os::unix::fs::symlink;
        let directory = Directory::new();
        let policy = WritePolicy::new([]).unwrap();
        let path = directory.0.join("original.ouscene");
        fs::write(&path, FIXTURE).unwrap();
        let final_link = directory.0.join("link.ouscene");
        symlink(&path, &final_link).unwrap();
        let parent_link = directory.0.join("parent-link");
        symlink(&directory.0, &parent_link).unwrap();
        let scene = decode(FIXTURE.as_bytes()).unwrap();
        for linked in [
            final_link,
            parent_link.join("original.ouscene"),
            parent_link.join("new.ouscene"),
        ] {
            assert!(load(&linked, &policy).is_err());
            assert!(save(&linked, &scene, &policy).is_err());
        }
        assert_eq!(fs::read(&path).unwrap(), FIXTURE.as_bytes());
        assert!(!directory.0.join("new.ouscene").exists());
    }
}
