//! Independent scene hierarchy authoring (INFRA-EDITOR-001).
//!
//! Reparenting preserves the entity's local transform exactly; it does not
//! implement Unity world-position preservation. Undo restores scene snapshots,
//! so the document owner must preserve the session identity high-water mark.

use crate::history::History;
use crate::scene::{EntityId, Scene, SceneError};
use crate::selection::Selection;

pub use crate::hierarchy_revision::HierarchyRevision;

/// Return root identities in their authored order.
pub fn roots(scene: &Scene) -> Vec<EntityId> {
    scene.sibling_group(None).unwrap_or_default().to_vec()
}

/// Return direct child identities in their authored order.
///
/// A missing parent has no children and returns an empty list.
pub fn children(scene: &Scene, parent: EntityId) -> Vec<EntityId> {
    scene
        .sibling_group(Some(parent))
        .unwrap_or_default()
        .to_vec()
}

/// Change an entity's parent while preserving its exact local transform.
///
/// Return whether the document changed, even when history recording is disabled.
/// Missing entities or parents, self-parenting, cycles and excessive hierarchy
/// depth leave both scene and history unchanged. An unchanged parent is a no-op
/// that preserves any redo entries. A successful change is one history entry.
pub fn reparent(
    scene: &mut Scene,
    history: &mut History<Scene>,
    id: EntityId,
    parent: Option<EntityId>,
) -> Result<bool, SceneError> {
    if scene.get(id).is_some_and(|entity| entity.parent == parent) {
        return Ok(false);
    }
    let before = scene.clone();
    scene.edit(id, |entity| entity.parent = parent)?;
    history.commit("Reparent entity", before, scene.clone());
    Ok(true)
}

/// Remove an entity and its descendants as one scene history transaction.
///
/// Return removed identities in ascending order and prune selection after the
/// removal succeeds. A missing identity leaves scene, history and selection
/// unchanged. Undo restores scene contents, without restoring removed selection.
pub fn delete_subtree(
    scene: &mut Scene,
    history: &mut History<Scene>,
    selection: &mut Selection,
    id: EntityId,
) -> Result<Vec<EntityId>, SceneError> {
    let before = scene.clone();
    let removed = scene.remove_subtree(id)?;
    history.commit("Delete subtree", before, scene.clone());
    selection.prune(scene);
    Ok(removed)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scene::{Entity, Primitive, Transform};

    fn create(scene: &mut Scene, name: &str, parent: Option<EntityId>) -> EntityId {
        scene.create(name, Primitive::Empty, parent).unwrap()
    }

    #[test]
    fn queries_sort_roots_and_direct_children_by_identity() {
        let entity = |id: u64, parent: Option<u64>| Entity {
            id: EntityId(id),
            name: format!("Entity {id}"),
            parent: parent.map(EntityId),
            active: true,
            transform: Transform::default(),
            primitive: Primitive::Empty,
        };
        let scene = Scene::from_entities(
            "Hierarchy",
            9,
            vec![
                entity(8, Some(2)),
                entity(6, None),
                entity(4, Some(2)),
                entity(5, Some(4)),
                entity(2, None),
            ],
        )
        .unwrap();
        assert_eq!(roots(&scene), [EntityId(2), EntityId(6)]);
        assert_eq!(children(&scene, EntityId(2)), [EntityId(4), EntityId(8)]);
        assert_eq!(children(&scene, EntityId(4)), [EntityId(5)]);
        assert!(children(&scene, EntityId(99)).is_empty());
    }

    #[test]
    fn reparent_preserves_local_transform_and_is_one_undoable_action() {
        let mut scene = Scene::new("Hierarchy").unwrap();
        let parent = create(&mut scene, "Parent", None);
        let child = create(&mut scene, "Child", None);
        let transform = Transform {
            translation: [-0.0, 2.5, -9.0],
            rotation_deg: [90.0, -180.0, 13.0],
            scale: [-2.0, 0.0, 4.0],
        };
        scene
            .edit(child, |entity| entity.transform = transform)
            .unwrap();
        let before = scene.clone();
        let mut history = History::new(8);
        assert!(reparent(&mut scene, &mut history, child, Some(parent)).unwrap());
        let actual = scene.get(child).unwrap().transform;
        for (actual, expected) in actual
            .translation
            .into_iter()
            .chain(actual.rotation_deg)
            .chain(actual.scale)
            .zip(
                transform
                    .translation
                    .into_iter()
                    .chain(transform.rotation_deg)
                    .chain(transform.scale),
            )
        {
            assert_eq!(actual.to_bits(), expected.to_bits());
        }
        assert_eq!(scene.get(child).unwrap().parent, Some(parent));
        let after = scene.clone();
        assert_eq!(history.undo(&scene), Some(before));
        assert!(!history.can_undo());
        assert_eq!(history.redo(&scene), Some(after));
    }

    #[test]
    fn invalid_reparenting_preserves_document_and_redo() {
        let mut scene = Scene::new("Hierarchy").unwrap();
        let root = create(&mut scene, "Root", None);
        let child = create(&mut scene, "Child", Some(root));
        let leaf = create(&mut scene, "Leaf", Some(child));
        let mut history = History::new(8);
        reparent(&mut scene, &mut history, leaf, None).unwrap();
        scene = history.undo(&scene).unwrap();
        let before = scene.clone();
        for (id, parent) in [
            (root, Some(root)),
            (root, Some(leaf)),
            (root, Some(EntityId(99))),
            (EntityId(99), None),
        ] {
            assert!(reparent(&mut scene, &mut history, id, parent).is_err());
            assert_eq!(scene, before);
            assert!(!history.can_undo());
            assert_eq!(history.redo_label(), Some("Reparent entity"));
        }
        assert!(!reparent(&mut scene, &mut history, child, Some(root)).unwrap());
        assert_eq!(history.redo_label(), Some("Reparent entity"));
    }

    #[test]
    fn reparent_to_root_reports_change_with_history_disabled() {
        let mut scene = Scene::new("Hierarchy").unwrap();
        let root = create(&mut scene, "Root", None);
        let child = create(&mut scene, "Child", Some(root));
        let mut history = History::new(0);
        assert!(reparent(&mut scene, &mut history, child, None).unwrap());
        assert_eq!(roots(&scene), [root, child]);
        assert!(!history.can_undo());
    }

    #[test]
    fn deletion_prunes_selection_and_restores_whole_subtree_in_one_undo() {
        let mut scene = Scene::new("Hierarchy").unwrap();
        let root = create(&mut scene, "Root", None);
        let survivor = create(&mut scene, "Survivor", None);
        let child = create(&mut scene, "Child", Some(root));
        let leaf = create(&mut scene, "Leaf", Some(child));
        let mut selection = Selection::new();
        for id in [survivor, root, child, leaf] {
            selection.toggle(id, &scene).unwrap();
        }
        let before = scene.clone();
        let mut history = History::new(8);
        assert_eq!(
            delete_subtree(&mut scene, &mut history, &mut selection, root).unwrap(),
            [root, child, leaf]
        );
        assert_eq!(roots(&scene), [survivor]);
        assert_eq!(selection.ids().collect::<Vec<_>>(), [survivor]);
        assert_eq!(selection.primary(), Some(survivor));
        let after = scene.clone();
        assert_eq!(history.undo(&scene), Some(before));
        assert!(!history.can_undo());
        assert_eq!(history.redo(&scene), Some(after));
        assert_eq!(selection.ids().collect::<Vec<_>>(), [survivor]);
    }

    #[test]
    fn failed_deletion_preserves_even_stale_selection_and_redo() {
        let mut scene = Scene::new("Hierarchy").unwrap();
        let root = create(&mut scene, "Root", None);
        let child = create(&mut scene, "Child", Some(root));
        let mut history = History::new(8);
        reparent(&mut scene, &mut history, child, None).unwrap();
        scene = history.undo(&scene).unwrap();
        let mut selection = Selection::new();
        selection.replace(Some(child), &scene).unwrap();
        scene.remove_subtree(child).unwrap();
        let before = scene.clone();
        let selected = selection.clone();
        assert!(delete_subtree(&mut scene, &mut history, &mut selection, child).is_err());
        assert_eq!(scene, before);
        assert_eq!(selection, selected);
        assert!(!history.can_undo());
        assert_eq!(history.redo_label(), Some("Reparent entity"));
    }
}
