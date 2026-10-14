//! Complete validated archive state and exact serialized identity.

use std::collections::BTreeSet;

use engine_scene::ScenePlan;
use serde::{Deserialize, Serialize};

use crate::{hierarchy::HierarchyRevision, scene::EntityId};

use super::{ImportedError, ImportedSource, Result};
use validation::validate_plan;

mod document;
mod validation;

pub use document::ImportedDocument;

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
    // Process-local row-content identity; it is not part of the archive or semantic equality.
    #[serde(skip)]
    hierarchy_revision: HierarchyRevision,
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
            hierarchy_revision: HierarchyRevision::fresh(),
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
        let plan_identity = serde_json::to_vec(&self.plan)
            .map_err(|e| ImportedError::new("validate archive", e))?;
        let bytes =
            serde_json::to_vec(self).map_err(|e| ImportedError::new("validate archive", e))?;
        if bytes.len() > engine_scene::MAX_INPUT_BYTES {
            return Err(ImportedError::new(
                "validate archive",
                "archive exceeds 16 MiB",
            ));
        }

        self.plan_identity = plan_identity;
        self.hierarchy_revision = HierarchyRevision::fresh();
        Ok(())
    }
}
