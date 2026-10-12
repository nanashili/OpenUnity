use crate::{Result, SceneError, ScenePlan, parse_json, yaml};
use engine_assets::MetadataIndex;
use std::path::{Component, Path, PathBuf};

mod bindings;
mod document_values;
mod documents;
mod filesystem;
mod objects;
mod scripts;

use documents::DocumentConversion;
use filesystem::{confined, index, read};

/// Read actual scene/meta files and an explicit type/field binding manifest.
/// Paths must be within an absolute nonsymlink project root. No file is modified.
/// Only the reviewed authored grammar and exact skipped settings are supported.
pub fn import_project(root: &Path, scene: &Path, bindings: &Path) -> Result<ScenePlan> {
    if !root.is_absolute()
        || root
            .components()
            .any(|c| matches!(c, Component::ParentDir | Component::CurDir))
        || !root.is_dir()
    {
        return Err(SceneError::new("expected absolute project directory"));
    }
    let root = confined(root, root)?;
    let scene = confined(&root, scene)?;
    let binding_path = confined(&root, bindings)?;
    let mut metadata = MetadataIndex::new();
    index(&root, &root, 0, &mut 0, &mut metadata)?;
    let scene_meta = PathBuf::from(format!(
        "{}.meta",
        scene
            .to_str()
            .ok_or_else(|| SceneError::new("nonUnicode scene path"))?
    ));
    engine_assets::read(&scene_meta).map_err(SceneError::new)?;
    let scripts = bindings::load(&binding_path, &metadata)?;
    let settings = parse_json(include_bytes!("settings-defaults.json"))?;
    let mut conversion = DocumentConversion::new(&scene, &scripts, settings)?;
    for doc in yaml::parse(&read(&scene)?)? {
        conversion.push(doc)?;
    }
    conversion.finish()
}
