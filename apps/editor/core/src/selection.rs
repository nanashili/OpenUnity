//! Scene-scoped selection for the independent editor (INFRA-EDITOR-001).
//!
//! The document owner must call [`Selection::clear`] when opening or replacing
//! a scene. Entity identities are local to a document: checking that an ID
//! exists cannot distinguish equal IDs from different documents. After scene
//! removal, call [`Selection::prune`] before reading selection again.

use std::collections::BTreeSet;
use std::fmt;

use crate::scene::{EntityId, Scene};

/// A selection operation rejected an identity absent from the live scene.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SelectionError {
    operation: &'static str,
    id: EntityId,
}

impl fmt::Display for SelectionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{}: entity {} does not exist in the live scene",
            self.operation, self.id.0
        )
    }
}

impl std::error::Error for SelectionError {}

/// Ordered entity identities and an optional primary within one document.
///
/// The primary is always selected. Adding an identity makes it primary;
/// removing the primary chooses the smallest remaining identity.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Selection {
    ids: BTreeSet<EntityId>,
    primary: Option<EntityId>,
}

impl Selection {
    /// Create an empty selection.
    pub fn new() -> Self {
        Self::default()
    }

    /// Replace the selection with one live identity, or clear it with `None`.
    ///
    /// A zero or missing identity is rejected without changing selection.
    pub fn replace(&mut self, id: Option<EntityId>, scene: &Scene) -> Result<(), SelectionError> {
        if let Some(id) = id {
            Self::validate(id, scene, "replace selection")?;
        }
        self.ids.clear();
        self.ids.extend(id);
        self.primary = id;
        Ok(())
    }

    /// Toggle a live identity, pruning removed identities on success.
    ///
    /// An added identity becomes primary. Removing the primary selects the
    /// smallest remaining identity. Invalid identities leave selection intact.
    pub fn toggle(&mut self, id: EntityId, scene: &Scene) -> Result<(), SelectionError> {
        Self::validate(id, scene, "toggle selection")?;
        self.prune(scene);
        if self.ids.remove(&id) {
            self.repair_primary();
        } else {
            self.ids.insert(id);
            self.primary = Some(id);
        }
        Ok(())
    }

    /// Clear identities and primary, including when replacing a document.
    pub fn clear(&mut self) {
        self.ids.clear();
        self.primary = None;
    }

    /// Remove identities absent from the scene and repair the primary.
    ///
    /// This operates only within the current document scope; it cannot detect
    /// document replacement when another document contains the same IDs.
    pub fn prune(&mut self, scene: &Scene) {
        self.ids.retain(|id| scene.get(*id).is_some());
        self.repair_primary();
    }

    /// Selected identities in ascending order.
    pub fn ids(&self) -> impl ExactSizeIterator<Item = EntityId> + DoubleEndedIterator + '_ {
        self.ids.iter().copied()
    }

    /// Primary identity, or `None` when selection is empty.
    pub fn primary(&self) -> Option<EntityId> {
        self.primary
    }

    fn repair_primary(&mut self) {
        if self.primary.is_none_or(|id| !self.ids.contains(&id)) {
            self.primary = self.ids.first().copied();
        }
    }

    fn validate(
        id: EntityId,
        scene: &Scene,
        operation: &'static str,
    ) -> Result<(), SelectionError> {
        if id.0 == 0 || scene.get(id).is_none() {
            return Err(SelectionError { operation, id });
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scene::Primitive;

    #[test]
    fn toggling_keeps_order_and_repairs_primary_deterministically() {
        let mut scene = Scene::new("Selection").unwrap();
        let first = scene.create("First", Primitive::Empty, None).unwrap();
        let second = scene.create("Second", Primitive::Cube, None).unwrap();
        let third = scene.create("Third", Primitive::Camera, None).unwrap();
        let mut selection = Selection::new();
        for id in [third, first, second] {
            selection.toggle(id, &scene).unwrap();
            assert_eq!(selection.primary(), Some(id));
        }
        assert_eq!(selection.ids().collect::<Vec<_>>(), [first, second, third]);
        selection.toggle(third, &scene).unwrap();
        assert_eq!(selection.primary(), Some(second));
        selection.toggle(second, &scene).unwrap();
        assert_eq!(selection.primary(), Some(first));
        selection.toggle(first, &scene).unwrap();
        assert_eq!(selection.primary(), None);
        assert_eq!(selection.ids().len(), 0);
    }

    #[test]
    fn replacement_and_invalid_ids_are_transactional() {
        let mut scene = Scene::new("Selection").unwrap();
        let first = scene.create("First", Primitive::Empty, None).unwrap();
        let second = scene.create("Second", Primitive::Empty, None).unwrap();
        let mut selection = Selection::new();
        selection.toggle(first, &scene).unwrap();
        selection.toggle(second, &scene).unwrap();
        let before = selection.clone();
        for invalid in [EntityId(0), EntityId(999)] {
            assert!(selection.replace(Some(invalid), &scene).is_err());
            assert_eq!(selection, before);
            assert!(selection.toggle(invalid, &scene).is_err());
            assert_eq!(selection, before);
        }
        selection.replace(Some(first), &scene).unwrap();
        assert_eq!(selection.ids().collect::<Vec<_>>(), [first]);
        assert_eq!(selection.primary(), Some(first));
        selection.replace(None, &scene).unwrap();
        assert_eq!(selection, Selection::default());
    }

    #[test]
    fn subtree_deletion_prunes_selection_and_replacement_starts_empty() {
        let mut scene = Scene::new("Original").unwrap();
        let root = scene.create("Root", Primitive::Empty, None).unwrap();
        let child = scene.create("Child", Primitive::Empty, Some(root)).unwrap();
        let survivor = scene.create("Survivor", Primitive::Cube, None).unwrap();
        let mut selection = Selection::new();
        for id in [survivor, root, child] {
            selection.toggle(id, &scene).unwrap();
        }
        scene.remove_subtree(root).unwrap();
        selection.prune(&scene);
        assert_eq!(selection.ids().collect::<Vec<_>>(), [survivor]);
        assert_eq!(selection.primary(), Some(survivor));
        let new_id = scene.create("New", Primitive::Empty, None).unwrap();
        assert!(!selection.ids().any(|id| id == new_id));
        scene.remove_subtree(survivor).unwrap();
        selection.prune(&scene);
        assert_eq!(selection, Selection::default());
        selection.toggle(new_id, &scene).unwrap();

        let mut replacement = Scene::new("Replacement").unwrap();
        for _ in 0..new_id.0 {
            replacement
                .create("Different", Primitive::Empty, None)
                .unwrap();
        }
        assert!(replacement.get(new_id).is_some());
        selection.clear();
        selection.prune(&replacement);
        assert_eq!(selection, Selection::default());
    }
}
