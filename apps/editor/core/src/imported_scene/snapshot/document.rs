//! Active imported-document ownership, import, and scoped selection.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use engine_scene::ScenePlan;

use crate::{hierarchy::HierarchyRevision, history::History, scene::EntityId};

use super::super::{ImportedError, ImportedSource, Result};
use super::{ImportedEditorId, ImportedSnapshot};

mod queries;
mod transactions;

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
        let plan = engine_scene::import_project(project, scene, bindings)
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
    /// Opaque identity for the current imported hierarchy row content.
    pub fn hierarchy_revision(&self) -> HierarchyRevision {
        self.snapshot.hierarchy_revision.clone()
    }
    /// Exact retained scene plan; never an authoring projection.
    pub fn plan(&self) -> &ScenePlan {
        self.snapshot.plan()
    }
    /// Original source provenance and write exclusion.
    pub fn source(&self) -> &ImportedSource {
        self.snapshot.source()
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
}
