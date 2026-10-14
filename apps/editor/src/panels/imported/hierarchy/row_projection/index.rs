use std::collections::BTreeMap;

use openunity_editor_core::imported_scene::ImportedDocument;
use openunity_editor_core::scene::EntityId;

pub(super) struct ProjectionIndex {
    transform_by_file_id: BTreeMap<i64, usize>,
    game_object_by_file_id: BTreeMap<i64, usize>,
    entity_by_game_object_file_id: BTreeMap<i64, EntityId>,
}

impl ProjectionIndex {
    pub(super) fn new(document: &ImportedDocument) -> Result<Self, String> {
        let plan = document.plan();
        let transform_by_file_id = index_transforms(plan.transforms.iter().map(|row| row.file_id))?;
        let game_object_by_file_id =
            index_game_objects(plan.game_objects.iter().map(|row| row.file_id))?;
        let entity_by_game_object_file_id = index_editor_ids(document)?;

        Ok(Self {
            transform_by_file_id,
            game_object_by_file_id,
            entity_by_game_object_file_id,
        })
    }

    pub(super) fn resolve(
        &self,
        document: &ImportedDocument,
        transform_file_id: i64,
        expected_parent_file_id: i64,
    ) -> Result<(usize, usize, EntityId), String> {
        let transform_index = self
            .transform_by_file_id
            .get(&transform_file_id)
            .copied()
            .ok_or_else(|| {
                format!("Imported hierarchy is missing Transform file ID {transform_file_id}.")
            })?;
        let transform = &document.plan().transforms[transform_index];
        if transform.parent != expected_parent_file_id {
            return Err(format!(
                "Imported hierarchy parent mismatch for Transform file ID {transform_file_id}."
            ));
        }

        let game_object_index = self
            .game_object_by_file_id
            .get(&transform.owner)
            .copied()
            .ok_or_else(|| {
                format!(
                    "Imported hierarchy is missing GameObject file ID {}.",
                    transform.owner
                )
            })?;
        let entity_id = self
            .entity_by_game_object_file_id
            .get(&transform.owner)
            .copied()
            .ok_or_else(|| {
                format!(
                    "Imported hierarchy is missing the editor mapping for GameObject file ID {}.",
                    transform.owner
                )
            })?;

        Ok((transform_index, game_object_index, entity_id))
    }

    pub(super) fn ensure_complete(
        &self,
        document: &ImportedDocument,
        projected_count: usize,
    ) -> Result<(), String> {
        let expected = document.plan().game_objects.len();
        if projected_count != expected
            || self.transform_by_file_id.len() != expected
            || self.game_object_by_file_id.len() != expected
            || self.entity_by_game_object_file_id.len() != expected
        {
            return Err(format!(
                "Imported hierarchy projected {projected_count} of {expected} validated entities."
            ));
        }

        Ok(())
    }
}

fn index_transforms(file_ids: impl Iterator<Item = i64>) -> Result<BTreeMap<i64, usize>, String> {
    unique_indices(file_ids, "Transform")
}

fn index_game_objects(file_ids: impl Iterator<Item = i64>) -> Result<BTreeMap<i64, usize>, String> {
    unique_indices(file_ids, "GameObject")
}

fn unique_indices(
    file_ids: impl Iterator<Item = i64>,
    object_kind: &str,
) -> Result<BTreeMap<i64, usize>, String> {
    let mut index_by_file_id = BTreeMap::new();
    for (index, file_id) in file_ids.enumerate() {
        if index_by_file_id.insert(file_id, index).is_some() {
            return Err(format!(
                "Imported hierarchy has duplicate {object_kind} file ID {file_id}."
            ));
        }
    }

    Ok(index_by_file_id)
}

fn index_editor_ids(document: &ImportedDocument) -> Result<BTreeMap<i64, EntityId>, String> {
    let mut entity_by_game_object_file_id = BTreeMap::new();
    for mapping in document.snapshot().editor_ids() {
        if entity_by_game_object_file_id
            .insert(mapping.game_object_file_id, mapping.entity_id)
            .is_some()
        {
            return Err(format!(
                "Imported hierarchy has duplicate editor mapping for GameObject file ID {}.",
                mapping.game_object_file_id
            ));
        }
    }

    Ok(entity_by_game_object_file_id)
}
