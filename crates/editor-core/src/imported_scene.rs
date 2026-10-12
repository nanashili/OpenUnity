//! Lossless, independently owned authoring state for the reviewed saved-scene dialect.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::path::{Component, Path, PathBuf};

pub use openunity_scene::FieldValue;
use openunity_scene::{GameObjectPlan, ScenePlan, ScriptPlan, TransformPlan};
use serde::{Deserialize, Serialize};

use crate::{history::History, scene::EntityId};

/// A rejected import, archive or edit operation.
#[derive(Clone, Debug)]
pub struct ImportedError {
    /// Operation that failed.
    pub operation: &'static str,
    /// Owned explanation suitable for an editor diagnostic.
    pub message: String,
}
impl ImportedError {
    pub(crate) fn new(operation: &'static str, message: impl fmt::Display) -> Self {
        Self {
            operation,
            message: message.to_string(),
        }
    }
}
impl fmt::Display for ImportedError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.operation, self.message)
    }
}
impl std::error::Error for ImportedError {}
type Result<T> = std::result::Result<T, ImportedError>;

/// Read-only diagnostic provenance and an enduring source write exclusion.
#[derive(Clone, Debug, Deserialize, Serialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ImportedSource {
    /// Absolute normalized original source root; availability is not required on reopen.
    pub project_root: PathBuf,
    /// Relative scene source path.
    pub scene: PathBuf,
    /// Relative explicit binding manifest path.
    pub bindings: PathBuf,
}
impl ImportedSource {
    pub(crate) fn validate(&self) -> Result<()> {
        validate_path(&self.project_root, true)?;
        validate_path(&self.scene, false)?;
        validate_path(&self.bindings, false)
    }
    /// Additive lexical exclusion, including when the source root is absent.
    /// The caller must also use WritePolicy for actual checked filesystem writes.
    pub fn forbids_write(&self, path: &Path) -> Result<bool> {
        self.validate()?;
        let absolute = if path.is_absolute() {
            path.to_owned()
        } else {
            std::env::current_dir()
                .map_err(|e| ImportedError::new("check source write", e))?
                .join(path)
        };
        validate_path(&absolute, true)?;
        Ok(absolute.starts_with(&self.project_root))
    }
}
fn validate_path(path: &Path, absolute: bool) -> Result<()> {
    let value = path
        .to_str()
        .ok_or_else(|| ImportedError::new("validate provenance", "path is not UTF-8"))?;
    if value.is_empty()
        || value.len() > openunity_scene::MAX_STRING_BYTES
        || value.contains('\0')
        || path.is_absolute() != absolute
        || path
            .components()
            .any(|c| matches!(c, Component::ParentDir | Component::CurDir))
        || path.components().collect::<PathBuf>().to_str() != Some(value)
    {
        return Err(ImportedError::new(
            "validate provenance",
            "expected a normalized absolute root or nonempty relative source path",
        ));
    }
    Ok(())
}

/// One document handle, distinct from every scene-local component/file identity.
#[derive(Clone, Debug, Deserialize, Serialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ImportedEditorId {
    /// Positive document-local selection handle.
    pub entity_id: EntityId,
    /// Original GameObject file identity.
    pub game_object_file_id: i64,
}

/// Complete immutable imported archive state. No native objects or projection are stored.
#[derive(Clone, Debug, Serialize)]
pub struct ImportedSnapshot {
    format: &'static str,
    version: u32,
    source: ImportedSource,
    next_id: u64,
    editor_ids: Vec<ImportedEditorId>,
    plan: ScenePlan,
    // Canonical serialization distinguishes all finite float32 values, including signed zero.
    // It is derived only after validation, never supplied by an archive.
    #[serde(skip)]
    plan_identity: Vec<u8>,
}
impl PartialEq for ImportedSnapshot {
    fn eq(&self, other: &Self) -> bool {
        self.source == other.source
            && self.next_id == other.next_id
            && self.editor_ids == other.editor_ids
            && self.plan_identity == other.plan_identity
    }
}
impl ImportedSnapshot {
    pub(crate) fn new(
        source: ImportedSource,
        next_id: u64,
        editor_ids: Vec<ImportedEditorId>,
        plan: ScenePlan,
    ) -> Result<Self> {
        let mut snapshot = Self {
            format: "openunity.imported-scene",
            version: 1,
            source,
            next_id,
            editor_ids,
            plan,
            plan_identity: Vec::new(),
        };
        snapshot.refresh()?;
        Ok(snapshot)
    }
    /// Exact authoritative plan, including authored order and raw quaternion values.
    pub fn plan(&self) -> &ScenePlan {
        &self.plan
    }
    /// Original read-only provenance.
    pub fn source(&self) -> &ImportedSource {
        &self.source
    }
    /// Complete handle mapping in its retained archive order.
    pub fn editor_ids(&self) -> &[ImportedEditorId] {
        &self.editor_ids
    }
    /// Allocation high-water mark.
    pub fn next_id(&self) -> u64 {
        self.next_id
    }
    pub(crate) fn validate(&self) -> Result<()> {
        self.source.validate()?;
        validate_plan(&self.plan)?;
        if self.editor_ids.len() != self.plan.game_objects.len() {
            return Err(ImportedError::new(
                "validate archive",
                "incomplete editor mapping",
            ));
        }
        let expected: BTreeSet<_> = self.plan.game_objects.iter().map(|g| g.file_id).collect();
        let mut handles = BTreeSet::new();
        let mut objects = BTreeSet::new();
        for map in &self.editor_ids {
            if map.entity_id.0 == 0
                || map.entity_id.0 >= self.next_id
                || !handles.insert(map.entity_id)
                || !expected.contains(&map.game_object_file_id)
                || !objects.insert(map.game_object_file_id)
            {
                return Err(ImportedError::new(
                    "validate archive",
                    "invalid or duplicate editor mapping/counter",
                ));
            }
        }
        Ok(())
    }
    fn refresh(&mut self) -> Result<()> {
        self.validate()?;
        self.plan_identity = serde_json::to_vec(&self.plan)
            .map_err(|e| ImportedError::new("validate archive", e))?;
        let bytes =
            serde_json::to_vec(self).map_err(|e| ImportedError::new("validate archive", e))?;
        if bytes.len() > openunity_scene::MAX_INPUT_BYTES {
            return Err(ImportedError::new(
                "validate archive",
                "archive exceeds 16 MiB",
            ));
        }
        Ok(())
    }
}

fn field_kind(value: &FieldValue) -> u8 {
    match value {
        FieldValue::I32(_) => 0,
        FieldValue::F32(_) => 1,
        FieldValue::Bool(_) => 2,
        FieldValue::String(_) => 3,
        FieldValue::Vec3(_) => 4,
        FieldValue::Quat(_) => 5,
        FieldValue::Reference(_) => 6,
    }
}
fn validate_plan(plan: &ScenePlan) -> Result<()> {
    plan.validate()
        .map_err(|e| ImportedError::new("validate imported plan", e))?;
    let transforms: BTreeMap<_, _> = plan.transforms.iter().map(|t| (t.owner, t)).collect();
    for go in &plan.game_objects {
        if transforms
            .get(&go.file_id)
            .is_none_or(|t| go.components.first() != Some(&t.file_id))
        {
            return Err(ImportedError::new(
                "validate imported plan",
                "mandatory Transform must be first",
            ));
        }
    }
    for t in &plan.transforms {
        if t.local_rotation.iter().all(|v| *v == 0.0) {
            return Err(ImportedError::new(
                "validate imported plan",
                "Transform quaternion must be nonzero",
            ));
        }
    }
    let mut guids = BTreeMap::new();
    let mut types = BTreeMap::new();
    for script in &plan.scripts {
        if script.assembly != "Assembly-CSharp" {
            return Err(ImportedError::new(
                "validate imported plan",
                "only Assembly-CSharp bindings are supported",
            ));
        }
        let fields: BTreeMap<_, _> = script
            .fields
            .iter()
            .map(|f| (f.name.as_str(), field_kind(&f.value)))
            .collect();
        let identity = (
            script.assembly.as_str(),
            script.type_name.as_str(),
            script.execution_order,
            fields,
        );
        if guids
            .get(script.guid.as_str())
            .is_some_and(|prior| prior != &identity)
        {
            return Err(ImportedError::new(
                "validate imported plan",
                "conflicting GUID binding declarations",
            ));
        }
        let type_key = (script.assembly.as_str(), script.type_name.as_str());
        if types
            .get(&type_key)
            .is_some_and(|guid| *guid != script.guid.as_str())
        {
            return Err(ImportedError::new(
                "validate imported plan",
                "conflicting type/GUID identities",
            ));
        }
        types.insert(type_key, script.guid.as_str());
        guids.insert(script.guid.as_str(), identity);
    }
    Ok(())
}

/// Supported whole-entity edit; raw rotation components are never converted to Euler angles.
#[derive(Clone, Debug)]
pub struct ImportedEntityEdit {
    /// GameObject name, retaining the imported string bounds.
    pub name: String,
    /// Local GameObject activity state.
    pub active: bool,
    /// Finite local position components.
    pub local_position: [f32; 3],
    /// Finite nonzero raw quaternion components; no normalization.
    pub local_rotation: [f32; 4],
    /// Finite local scale components, including zero or negative values.
    pub local_scale: [f32; 3],
}
/// Borrowed canonical entity records.
#[derive(Clone, Copy, Debug)]
pub struct ImportedEntityView<'a> {
    /// Document-local owner handle.
    pub editor_id: EntityId,
    /// Original GameObject record.
    pub game_object: &'a GameObjectPlan,
    /// Original mandatory Transform record.
    pub transform: &'a TransformPlan,
}
/// Component inspection follows the original GameObject component order.
#[derive(Clone, Copy, Debug)]
pub enum ImportedComponent<'a> {
    /// Original mandatory Transform target.
    Transform(&'a TransformPlan),
    /// Original declaratively bound script target.
    Script(&'a ScriptPlan),
}
/// Kind of a canonical file-local reference target.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ImportedReferenceKind {
    /// Original GameObject target.
    GameObject,
    /// Original mandatory Transform target.
    Transform,
    /// Original declaratively bound script target.
    Script,
}
/// Exact target record, with no native identity.
#[derive(Clone, Copy, Debug)]
pub enum ImportedReferenceTarget<'a> {
    /// Original GameObject target.
    GameObject(&'a GameObjectPlan),
    /// Original mandatory Transform target.
    Transform(&'a TransformPlan),
    /// Original declaratively bound script target.
    Script(&'a ScriptPlan),
}
/// Resolved borrowed reference and its canonical owner handle.
#[derive(Clone, Copy, Debug)]
pub struct ImportedReference<'a> {
    /// Exact local file identity.
    pub file_id: i64,
    /// Canonical owning GameObject editor handle.
    pub owner: EntityId,
    /// Typed target kind.
    pub kind: ImportedReferenceKind,
    /// GameObject name, retaining the imported string bounds.
    pub name: &'a str,
    /// Borrowed canonical target record.
    pub target: ImportedReferenceTarget<'a>,
}

/// Active imported document with private bounded history and scoped selection.
#[derive(Clone, Debug)]
pub struct ImportedDocument {
    snapshot: ImportedSnapshot,
    history: History<ImportedSnapshot>,
    saved: Option<ImportedSnapshot>,
    selection: BTreeSet<EntityId>,
    primary: Option<EntityId>,
}
impl ImportedDocument {
    /// Import the actual bounded source dialect without executing C# or allocating a runtime.
    pub fn import_project(
        project: &Path,
        scene: &Path,
        bindings: &Path,
        history_capacity: usize,
    ) -> Result<Self> {
        let plan = openunity_scene::import_project(project, scene, bindings)
            .map_err(|e| ImportedError::new("import scene", e))?;
        let root =
            std::fs::canonicalize(project).map_err(|e| ImportedError::new("import scene", e))?;
        let relative = |p: &Path| -> Result<PathBuf> {
            if p.is_absolute() {
                p.strip_prefix(&root)
                    .map(Path::to_path_buf)
                    .map_err(|e| ImportedError::new("import scene", e))
            } else {
                Ok(p.to_path_buf())
            }
        };
        let source = ImportedSource {
            project_root: root.clone(),
            scene: relative(scene)?,
            bindings: relative(bindings)?,
        };
        let editor_ids = plan
            .game_objects
            .iter()
            .enumerate()
            .map(|(i, g)| ImportedEditorId {
                entity_id: EntityId(i as u64 + 1),
                game_object_file_id: g.file_id,
            })
            .collect();
        let next_id = u64::try_from(plan.game_objects.len())
            .map_err(|e| ImportedError::new("import scene", e))?
            .checked_add(1)
            .ok_or_else(|| ImportedError::new("import scene", "editor identity overflow"))?;
        Self::from_snapshot(
            ImportedSnapshot::new(source, next_id, editor_ids, plan)?,
            history_capacity,
        )
    }
    /// Create an unsaved independent document; archive-open callers mark it saved after installation.
    pub fn from_snapshot(snapshot: ImportedSnapshot, history_capacity: usize) -> Result<Self> {
        snapshot.validate()?;
        Ok(Self {
            snapshot,
            history: History::new(history_capacity),
            saved: None,
            selection: BTreeSet::new(),
            primary: None,
        })
    }
    /// Complete authoritative immutable archive state.
    pub fn snapshot(&self) -> &ImportedSnapshot {
        &self.snapshot
    }
    /// Exact retained scene plan; never an authoring projection.
    pub fn plan(&self) -> &ScenePlan {
        self.snapshot.plan()
    }
    /// Original source provenance and write exclusion.
    pub fn source(&self) -> &ImportedSource {
        self.snapshot.source()
    }
    fn handle(&self, file_id: i64) -> Result<EntityId> {
        self.snapshot
            .editor_ids
            .iter()
            .find(|m| m.game_object_file_id == file_id)
            .map(|m| m.entity_id)
            .ok_or_else(|| {
                ImportedError::new("resolve imported identity", "missing GameObject mapping")
            })
    }
    /// Root owner handles in original Transform root order.
    pub fn roots(&self) -> Vec<EntityId> {
        self.plan()
            .roots
            .iter()
            .filter_map(|id| self.plan().transforms.iter().find(|t| t.file_id == *id))
            .filter_map(|t| self.handle(t.owner).ok())
            .collect()
    }
    /// Resolve a canonical owner and its mandatory Transform.
    pub fn entity(&self, id: EntityId) -> Result<ImportedEntityView<'_>> {
        let file = self
            .snapshot
            .editor_ids
            .iter()
            .find(|m| m.entity_id == id)
            .ok_or_else(|| ImportedError::new("inspect imported entity", "missing editor handle"))?
            .game_object_file_id;
        let game_object = self
            .plan()
            .game_objects
            .iter()
            .find(|g| g.file_id == file)
            .ok_or_else(|| ImportedError::new("inspect imported entity", "missing GameObject"))?;
        let transform = self
            .plan()
            .transforms
            .iter()
            .find(|t| t.owner == file)
            .ok_or_else(|| ImportedError::new("inspect imported entity", "missing Transform"))?;
        Ok(ImportedEntityView {
            editor_id: id,
            game_object,
            transform,
        })
    }
    /// Child owner handles in original authored child order.
    pub fn children(&self, id: EntityId) -> Result<Vec<EntityId>> {
        self.entity(id)?
            .transform
            .children
            .iter()
            .map(|child| {
                let t = self
                    .plan()
                    .transforms
                    .iter()
                    .find(|t| t.file_id == *child)
                    .ok_or_else(|| {
                        ImportedError::new("inspect imported children", "missing Transform")
                    })?;
                self.handle(t.owner)
            })
            .collect()
    }
    /// Borrow all authored components in their original order.
    pub fn components(&self, id: EntityId) -> Result<Vec<ImportedComponent<'_>>> {
        self.entity(id)?
            .game_object
            .components
            .iter()
            .map(|file| {
                if let Some(t) = self.plan().transforms.iter().find(|t| t.file_id == *file) {
                    return Ok(ImportedComponent::Transform(t));
                }
                self.plan()
                    .scripts
                    .iter()
                    .find(|s| s.file_id == *file)
                    .map(ImportedComponent::Script)
                    .ok_or_else(|| {
                        ImportedError::new("inspect imported components", "missing component")
                    })
            })
            .collect()
    }
    /// Resolve a local file identity; zero is the explicit null reference.
    pub fn resolve_reference(&self, file_id: i64) -> Result<Option<ImportedReference<'_>>> {
        if file_id == 0 {
            return Ok(None);
        }
        let (owner_file, kind, target) = if let Some(g) = self
            .plan()
            .game_objects
            .iter()
            .find(|g| g.file_id == file_id)
        {
            (
                g.file_id,
                ImportedReferenceKind::GameObject,
                ImportedReferenceTarget::GameObject(g),
            )
        } else if let Some(t) = self.plan().transforms.iter().find(|t| t.file_id == file_id) {
            (
                t.owner,
                ImportedReferenceKind::Transform,
                ImportedReferenceTarget::Transform(t),
            )
        } else if let Some(s) = self.plan().scripts.iter().find(|s| s.file_id == file_id) {
            (
                s.owner,
                ImportedReferenceKind::Script,
                ImportedReferenceTarget::Script(s),
            )
        } else {
            return Err(ImportedError::new(
                "resolve imported reference",
                "unresolved file identity",
            ));
        };
        let owner = self.handle(owner_file)?;
        Ok(Some(ImportedReference {
            file_id,
            owner,
            kind,
            target,
            name: &self.entity(owner)?.game_object.name,
        }))
    }
    /// Replace or toggle selection after validating the document-local handle.
    pub fn select(&mut self, id: Option<EntityId>, toggle: bool) -> Result<()> {
        if let Some(id) = id {
            self.entity(id)?;
            if !toggle {
                self.selection.clear();
                self.selection.insert(id);
                self.primary = Some(id);
            } else if self.selection.remove(&id) {
                if self.primary == Some(id) {
                    self.primary = self.selection.first().copied();
                }
            } else {
                self.selection.insert(id);
                self.primary = Some(id);
            }
        } else {
            self.selection.clear();
            self.primary = None;
        }
        Ok(())
    }
    /// Currently selected document-local handles.
    pub fn selection(&self) -> &BTreeSet<EntityId> {
        &self.selection
    }
    /// Primary selected handle, if any.
    pub fn primary(&self) -> Option<EntityId> {
        self.primary
    }
    fn commit(&mut self, label: &str, mut candidate: ImportedSnapshot) -> Result<bool> {
        candidate.refresh()?;
        if candidate == self.snapshot {
            return Ok(false);
        }
        self.history
            .commit(label, self.snapshot.clone(), candidate.clone());
        self.snapshot = candidate;
        Ok(true)
    }
    /// Validate a cloned complete state and commit one supported entity edit.
    pub fn edit_entity(&mut self, id: EntityId, edit: ImportedEntityEdit) -> Result<bool> {
        let file = self.entity(id)?.game_object.file_id;
        let mut candidate = self.snapshot.clone();
        let g = candidate
            .plan
            .game_objects
            .iter_mut()
            .find(|g| g.file_id == file)
            .ok_or_else(|| ImportedError::new("edit imported entity", "missing GameObject"))?;
        g.name = edit.name;
        g.active = edit.active;
        let t = candidate
            .plan
            .transforms
            .iter_mut()
            .find(|t| t.owner == file)
            .ok_or_else(|| ImportedError::new("edit imported entity", "missing Transform"))?;
        t.local_position = edit.local_position;
        t.local_rotation = edit.local_rotation;
        t.local_scale = edit.local_scale;
        self.commit("Edit imported entity", candidate)
    }
    /// Validate and commit an existing script enabled-state edit.
    pub fn set_script_enabled(&mut self, file_id: i64, enabled: bool) -> Result<bool> {
        let mut candidate = self.snapshot.clone();
        let script = candidate
            .plan
            .scripts
            .iter_mut()
            .find(|s| s.file_id == file_id)
            .ok_or_else(|| {
                ImportedError::new("edit imported script", "missing script file identity")
            })?;
        script.enabled = enabled;
        self.commit("Set imported script enabled", candidate)
    }
    /// Whether a committed transaction can be undone.
    pub fn can_undo(&self) -> bool {
        self.history.can_undo()
    }
    /// Whether an undone transaction can be restored.
    pub fn can_redo(&self) -> bool {
        self.history.can_redo()
    }
    /// Label of the next undo transaction.
    pub fn undo_label(&self) -> Option<&str> {
        self.history.undo_label()
    }
    /// Label of the next redo transaction.
    pub fn redo_label(&self) -> Option<&str> {
        self.history.redo_label()
    }
    /// Restore the preceding whole-state snapshot without changing selection.
    pub fn undo(&mut self) -> Result<bool> {
        if let Some(mut snapshot) = self.history.undo(&self.snapshot) {
            snapshot.next_id = snapshot.next_id.max(self.snapshot.next_id);
            self.snapshot = snapshot;
            Ok(true)
        } else {
            Ok(false)
        }
    }
    /// Restore the following whole-state snapshot without changing selection.
    pub fn redo(&mut self) -> Result<bool> {
        if let Some(mut snapshot) = self.history.redo(&self.snapshot) {
            snapshot.next_id = snapshot.next_id.max(self.snapshot.next_id);
            self.snapshot = snapshot;
            Ok(true)
        } else {
            Ok(false)
        }
    }
    /// Whether the exact state differs from the last successful save.
    pub fn is_dirty(&self) -> bool {
        self.saved.as_ref() != Some(&self.snapshot)
    }
    /// Record the current exact state after a successful independent save.
    pub fn mark_saved(&mut self) {
        self.saved = Some(self.snapshot.clone());
    }
}
