//! Validated inspector edits for the independent editor (INFRA-EDITOR-001).

use crate::history::History;
use crate::scene::{EntityId, Scene, SceneError, Transform};

/// Editable inspector fields applied together as one user action.
///
/// Identity, hierarchy and primitive representation are not inspector fields.
#[derive(Clone, Debug, PartialEq)]
pub struct EntityEdit {
    /// Nonempty entity name, bounded by [`crate::scene::MAX_NAME_BYTES`].
    pub name: String,
    /// Whether the entity is active.
    pub active: bool,
    /// Local transform; all components must be finite.
    pub transform: Transform,
}

/// Applies validated inspector fields and records one `Edit entity` transaction.
///
/// Returns whether the scene changed, even when history recording is disabled.
/// Missing entities and invalid fields leave both scene and history unchanged.
/// A no-op preserves existing undo and redo transactions.
pub fn apply(
    scene: &mut Scene,
    history: &mut History<Scene>,
    id: EntityId,
    edit: EntityEdit,
) -> Result<bool, SceneError> {
    let before = scene.clone();
    scene.edit(id, |entity| {
        entity.name = edit.name;
        entity.active = edit.active;
        entity.transform = edit.transform;
    })?;
    if *scene == before {
        return Ok(false);
    }
    history.commit("Edit entity", before, scene.clone());
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::{EntityEdit, apply};
    use crate::history::History;
    use crate::scene::{EntityId, MAX_NAME_BYTES, Primitive, Scene, Transform};

    fn fixture() -> (Scene, EntityId, History<Scene>) {
        let mut scene = Scene::new("Inspector").unwrap();
        let id = scene.create("Original", Primitive::Cube, None).unwrap();
        (scene, id, History::new(8))
    }

    fn edit() -> EntityEdit {
        EntityEdit {
            name: "Changed".into(),
            active: false,
            transform: Transform {
                translation: [1.0, 2.0, 3.0],
                rotation_deg: [-90.0, 180.0, 45.0],
                scale: [0.0, -1.0, 2.0],
            },
        }
    }

    #[test]
    fn one_undo_restores_all_fields_without_changing_identity_or_hierarchy() {
        let (mut scene, parent, mut history) = fixture();
        let id = scene
            .create("Child", Primitive::Camera, Some(parent))
            .unwrap();
        let before = scene.clone();
        let proposed = edit();

        assert!(apply(&mut scene, &mut history, id, proposed.clone()).unwrap());
        let entity = scene.get(id).unwrap();
        assert_eq!(entity.name, proposed.name);
        assert_eq!(entity.active, proposed.active);
        assert_eq!(entity.transform, proposed.transform);
        assert_eq!(entity.id, id);
        assert_eq!(entity.parent, Some(parent));
        assert_eq!(entity.primitive, Primitive::Camera);
        assert_eq!(history.undo_label(), Some("Edit entity"));
        let after = scene.clone();
        scene = history.undo(&scene).unwrap();
        assert_eq!(scene, before);
        assert!(!history.can_undo());
        assert_eq!(history.redo(&scene), Some(after));
    }

    #[test]
    fn rejects_each_nonfinite_transform_component_atomically() {
        let (mut scene, id, mut history) = fixture();
        let before = scene.clone();
        for invalid in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
            for field in 0..3 {
                for axis in 0..3 {
                    let mut proposed = edit();
                    match field {
                        0 => proposed.transform.translation[axis] = invalid,
                        1 => proposed.transform.rotation_deg[axis] = invalid,
                        _ => proposed.transform.scale[axis] = invalid,
                    }
                    assert!(apply(&mut scene, &mut history, id, proposed).is_err());
                    assert_eq!(scene, before);
                    assert!(!history.can_undo());
                    assert!(!history.can_redo());
                }
            }
        }
    }

    #[test]
    fn rejects_invalid_names_and_accepts_the_utf8_byte_limit() {
        let (mut scene, id, mut history) = fixture();
        let before = scene.clone();
        for name in [String::new(), "é".repeat(MAX_NAME_BYTES / 2 + 1)] {
            let mut proposed = edit();
            proposed.name = name;
            assert!(apply(&mut scene, &mut history, id, proposed).is_err());
            assert_eq!(scene, before);
            assert!(!history.can_undo());
        }
        let mut proposed = edit();
        proposed.name = "é".repeat(MAX_NAME_BYTES / 2);
        assert!(apply(&mut scene, &mut history, id, proposed).unwrap());
        assert_eq!(scene.get(id).unwrap().name.len(), MAX_NAME_BYTES);
    }

    #[test]
    fn missing_entity_preserves_scene_and_history() {
        let (mut scene, _, mut history) = fixture();
        let before = scene.clone();
        let error = apply(&mut scene, &mut history, EntityId(999), edit()).unwrap_err();
        assert_eq!(error.operation(), "edit entity");
        assert!(error.detail().contains("999"));
        assert_eq!(scene, before);
        assert!(!history.can_undo());
        assert!(!history.can_redo());
    }

    #[test]
    fn noop_and_failed_edits_preserve_redo() {
        let (mut scene, id, mut history) = fixture();
        assert!(apply(&mut scene, &mut history, id, edit()).unwrap());
        scene = history.undo(&scene).unwrap();
        let before = scene.clone();
        let entity = scene.get(id).unwrap();
        let unchanged = EntityEdit {
            name: entity.name.clone(),
            active: entity.active,
            transform: entity.transform,
        };
        assert!(!apply(&mut scene, &mut history, id, unchanged).unwrap());
        let mut invalid = edit();
        invalid.name.clear();
        assert!(apply(&mut scene, &mut history, id, invalid).is_err());
        assert_eq!(scene, before);
        assert!(!history.can_undo());
        assert_eq!(history.redo_label(), Some("Edit entity"));
        let restored = history.redo(&scene).unwrap();
        assert_eq!(restored.get(id).unwrap().name, "Changed");
    }

    #[test]
    fn change_is_reported_when_history_is_disabled() {
        let (mut scene, id, _) = fixture();
        let mut history = History::new(0);
        assert!(apply(&mut scene, &mut history, id, edit()).unwrap());
        assert_eq!(scene.get(id).unwrap().name, "Changed");
        assert!(!history.can_undo());
    }
}
