//! Independently designed, bounded saved-scene import and native transfer plan.
//! Only the reviewed authored text dialect is accepted; unsupported input fails.
#![forbid(unsafe_code)]

use openunity_assets::Guid;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

mod importer;
mod json;
mod yaml;
pub use importer::import_project;

/// Maximum scene, binding manifest, or transfer payload bytes.
pub const MAX_INPUT_BYTES: usize = 16 * 1024 * 1024;
/// Maximum documents, graph objects, or discovered project entries.
pub const MAX_OBJECTS: usize = 10_000;
/// Maximum parser or hierarchy depth.
pub const MAX_DEPTH: usize = 64;
/// Maximum UTF-8 bytes in one string.
pub const MAX_STRING_BYTES: usize = 1024 * 1024;
/// Maximum serialized fields on one script.
pub const MAX_FIELDS: usize = 256;

/// Explicit malformed, unsupported, or inaccessible scene input.
#[derive(Debug)]
pub struct SceneError(String);
impl SceneError {
    pub(crate) fn new(detail: impl ToString) -> Self {
        Self(detail.to_string())
    }
}
impl fmt::Display for SceneError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}
impl std::error::Error for SceneError {}
type Result<T> = std::result::Result<T, SceneError>;

/// Owned scene graph, with local file identities distinct from native identities.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ScenePlan {
    /// Transfer format version, currently one.
    pub schema_version: u32,
    /// Diagnostic name, not a managed type or native identity.
    pub scene_name: String,
    /// Ordered root Transform file identities.
    pub roots: Vec<i64>,
    /// GameObject records.
    pub game_objects: Vec<GameObjectPlan>,
    /// Mandatory Transform records.
    pub transforms: Vec<TransformPlan>,
    /// Authored managed component records.
    pub scripts: Vec<ScriptPlan>,
}

/// Serialized GameObject state.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct GameObjectPlan {
    /// Local file identity.
    pub file_id: i64,
    /// Object name.
    pub name: String,
    /// Local activity flag.
    pub active: bool,
    /// Ordered component file identities, including the mandatory Transform.
    pub components: Vec<i64>,
}

/// Serialized local Transform state.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TransformPlan {
    /// Local file identity.
    pub file_id: i64,
    /// Owning GameObject file identity.
    pub owner: i64,
    /// Parent Transform file identity, or zero for a root.
    pub parent: i64,
    /// Ordered child Transform file identities.
    pub children: Vec<i64>,
    /// Local position, requiring finite float32 components.
    pub local_position: [f32; 3],
    /// Local rotation; native numerical constraints additionally apply.
    pub local_rotation: [f32; 4],
    /// Local scale; native numerical constraints additionally apply.
    pub local_scale: [f32; 3],
}

/// An explicitly GUID-bound managed component.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ScriptPlan {
    /// Local file identity.
    pub file_id: i64,
    /// Owning GameObject file identity.
    pub owner: i64,
    /// Validated metadata GUID.
    pub guid: String,
    /// Independently compiled managed assembly identity.
    pub assembly: String,
    /// Fully qualified managed type identity.
    pub type_name: String,
    /// Serialized Behaviour.enabled flag.
    pub enabled: bool,
    /// Explicit script execution order, checked against reflected metadata.
    pub execution_order: i32,
    /// Complete supported serialized instance fields.
    pub fields: Vec<FieldPlan>,
}

/// One strictly typed serialized field.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FieldPlan {
    /// Exact reflected field name.
    pub name: String,
    /// Typed value, without property setters or numeric coercion.
    pub value: FieldValue,
}

/// Supported independent transfer kinds; quaternion fields are not normalized.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "kind", content = "value", deny_unknown_fields)]
pub enum FieldValue {
    /// Signed 32-bit integer.
    #[serde(rename = "i32")]
    I32(i32),
    /// Finite 32-bit float.
    #[serde(rename = "f32")]
    F32(f32),
    /// Boolean.
    #[serde(rename = "bool")]
    Bool(bool),
    /// UTF-8 string.
    #[serde(rename = "string")]
    String(String),
    /// Three finite float components.
    #[serde(rename = "vec3")]
    Vec3([f32; 3]),
    /// Four finite raw float components.
    #[serde(rename = "quat")]
    Quat([f32; 4]),
    /// Local file identity, with zero explicitly denoting null.
    #[serde(rename = "reference")]
    Reference(i64),
}

pub(crate) fn text(value: &str) -> Result<()> {
    if value.len() > MAX_STRING_BYTES || value.contains('\0') {
        return Err(SceneError::new("oversized string or embedded NUL"));
    }
    Ok(())
}
fn unique_ids(ids: impl IntoIterator<Item = i64>) -> Result<BTreeSet<i64>> {
    let mut set = BTreeSet::new();
    for id in ids {
        if id <= 0 || !set.insert(id) {
            return Err(SceneError::new("invalid or duplicate fileID"));
        }
    }
    Ok(set)
}

impl ScenePlan {
    /// Validate bounded identities, ownership, ordering, references and numbers.
    /// Native Transform admissibility and managed reflection are separate checks.
    pub fn validate(&self) -> Result<()> {
        if self.schema_version != 1 {
            return Err(SceneError::new("unsupported plan schema"));
        }
        text(&self.scene_name)?;
        let count = self
            .game_objects
            .len()
            .checked_add(self.transforms.len())
            .and_then(|n| n.checked_add(self.scripts.len()))
            .ok_or_else(|| SceneError::new("object count overflow"))?;
        if count == 0 || count > MAX_OBJECTS || self.transforms.len() != self.game_objects.len() {
            return Err(SceneError::new(
                "invalid graph size or mandatory Transform count",
            ));
        }
        let all = unique_ids(
            self.game_objects
                .iter()
                .map(|o| o.file_id)
                .chain(self.transforms.iter().map(|o| o.file_id))
                .chain(self.scripts.iter().map(|o| o.file_id)),
        )?;
        let gos: BTreeMap<_, _> = self.game_objects.iter().map(|o| (o.file_id, o)).collect();
        let transforms: BTreeMap<_, _> = self.transforms.iter().map(|o| (o.file_id, o)).collect();
        let mut owner = BTreeMap::new();
        for tr in &self.transforms {
            if !gos.contains_key(&tr.owner) {
                return Err(SceneError::new("unresolved Transform owner"));
            }
            owner.insert(tr.file_id, tr.owner);
            if !tr
                .local_position
                .iter()
                .chain(&tr.local_rotation)
                .chain(&tr.local_scale)
                .all(|f| f.is_finite())
            {
                return Err(SceneError::new("nonfinite Transform value"));
            }
            if tr.parent != 0 && !transforms.contains_key(&tr.parent) {
                return Err(SceneError::new("unresolved parent"));
            }
            unique_ids(tr.children.iter().copied())?;
            for child in &tr.children {
                if transforms.get(child).is_none_or(|c| c.parent != tr.file_id) {
                    return Err(SceneError::new("parent/child disagreement"));
                }
            }
            let mut ancestor = tr.file_id;
            let mut visited = BTreeSet::new();
            while ancestor != 0 {
                if !visited.insert(ancestor) || visited.len() > MAX_DEPTH {
                    return Err(SceneError::new("hierarchy cycle or depth limit"));
                }
                let node = transforms
                    .get(&ancestor)
                    .ok_or_else(|| SceneError::new("unresolved ancestor"))?;
                if node.parent != 0 && !transforms[&node.parent].children.contains(&ancestor) {
                    return Err(SceneError::new("missing child link"));
                }
                ancestor = node.parent;
            }
        }
        let roots = unique_ids(self.roots.iter().copied())?;
        let expected: BTreeSet<_> = self
            .transforms
            .iter()
            .filter(|t| t.parent == 0)
            .map(|t| t.file_id)
            .collect();
        if roots != expected {
            return Err(SceneError::new("root order membership mismatch"));
        }
        for script in &self.scripts {
            let guid = script.guid.parse::<Guid>().map_err(SceneError::new)?;
            if guid.to_string() != script.guid {
                return Err(SceneError::new("script GUID must be canonical lower-case"));
            }
            text(&script.assembly)?;
            text(&script.type_name)?;
            if script.assembly.is_empty()
                || script.type_name.is_empty()
                || !gos.contains_key(&script.owner)
                || script.fields.len() > MAX_FIELDS
            {
                return Err(SceneError::new("invalid script binding/owner/fields"));
            }
            owner.insert(script.file_id, script.owner);
            let mut names = BTreeSet::new();
            for field in &script.fields {
                text(&field.name)?;
                if field.name.is_empty() || !names.insert(&field.name) {
                    return Err(SceneError::new("invalid or duplicate field"));
                }
                match &field.value {
                    FieldValue::F32(v) if !v.is_finite() => {
                        return Err(SceneError::new("nonfinite field"));
                    }
                    FieldValue::Vec3(v) if !v.iter().all(|v| v.is_finite()) => {
                        return Err(SceneError::new("nonfinite vector field"));
                    }
                    FieldValue::Quat(v) if !v.iter().all(|v| v.is_finite()) => {
                        return Err(SceneError::new("nonfinite quaternion field"));
                    }
                    FieldValue::String(v) => text(v)?,
                    FieldValue::Reference(id) if *id != 0 && !all.contains(id) => {
                        return Err(SceneError::new("unresolved field reference"));
                    }
                    _ => (),
                }
            }
        }
        let mut listed = BTreeSet::new();
        for go in &self.game_objects {
            text(&go.name)?;
            if go.components.is_empty() || go.components.len() > MAX_OBJECTS {
                return Err(SceneError::new("component list limit"));
            }
            let mut mandatory = 0;
            for component in &go.components {
                if owner.get(component) != Some(&go.file_id) || !listed.insert(*component) {
                    return Err(SceneError::new("component owner/list disagreement"));
                }
                mandatory += usize::from(transforms.contains_key(component));
            }
            if mandatory != 1 {
                return Err(SceneError::new("mandatory Transform missing/duplicated"));
            }
        }
        if listed.len() != owner.len() {
            return Err(SceneError::new("unlisted component"));
        }
        Ok(())
    }
}

/// Parse a strict bounded UTF-8 JSON plan, rejecting duplicate and unknown keys.
pub fn parse_plan(bytes: &[u8]) -> Result<ScenePlan> {
    let value = json::parse(bytes)?;
    let plan: ScenePlan = serde_json::from_value(value).map_err(SceneError::new)?;
    plan.validate()?;
    Ok(plan)
}
