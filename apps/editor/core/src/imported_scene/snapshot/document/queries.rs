//! Authored-order hierarchy, component, and reference queries.

use crate::scene::EntityId;

use super::super::super::{
    ImportedComponent, ImportedEntityView, ImportedError, ImportedReference, ImportedReferenceKind,
    ImportedReferenceTarget, Result,
};
use super::ImportedDocument;

impl ImportedDocument {
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
}
