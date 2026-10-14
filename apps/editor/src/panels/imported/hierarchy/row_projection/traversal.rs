use std::collections::BTreeSet;

use openunity_editor_core::imported_scene::ImportedDocument;
use openunity_editor_core::scene::EntityId;

use super::super::super::super::hierarchy_browser::Row;
use super::index::ProjectionIndex;

struct PendingRow {
    transform_file_id: i64,
    expected_parent_file_id: i64,
    parent: Option<EntityId>,
    depth: usize,
}

pub(super) fn preorder_rows(
    document: &ImportedDocument,
    index: &ProjectionIndex,
) -> Result<Vec<Row>, String> {
    let plan = document.plan();
    let mut pending = pending_roots(plan.roots.iter().copied(), plan.transforms.len());
    let mut visited_transforms = BTreeSet::new();
    let mut visited_entities = BTreeSet::new();
    let mut rows = Vec::with_capacity(plan.game_objects.len());

    while let Some(pending_row) = pending.pop() {
        if !visited_transforms.insert(pending_row.transform_file_id) {
            return Err(format!(
                "Imported hierarchy visits Transform file ID {} more than once.",
                pending_row.transform_file_id
            ));
        }

        let (transform_index, game_object_index, entity_id) = index.resolve(
            document,
            pending_row.transform_file_id,
            pending_row.expected_parent_file_id,
        )?;
        if !visited_entities.insert(entity_id) {
            return Err(format!(
                "Imported hierarchy visits editor ID {} more than once.",
                entity_id.0
            ));
        }

        let transform = &plan.transforms[transform_index];
        let game_object = &plan.game_objects[game_object_index];
        rows.push(Row {
            id: entity_id,
            parent: pending_row.parent,
            name: game_object.name.clone(),
            depth: pending_row.depth,
            active: game_object.active,
            sibling_position: None,
            root_position: None,
        });

        push_children(
            &mut pending,
            transform.file_id,
            &transform.children,
            entity_id,
            pending_row.depth,
        )?;
    }

    Ok(rows)
}

fn pending_roots(
    roots: impl DoubleEndedIterator<Item = i64>,
    transform_count: usize,
) -> Vec<PendingRow> {
    let mut pending = Vec::with_capacity(transform_count);
    for root_file_id in roots.rev() {
        pending.push(PendingRow {
            transform_file_id: root_file_id,
            expected_parent_file_id: 0,
            parent: None,
            depth: 0,
        });
    }

    pending
}

fn push_children(
    pending: &mut Vec<PendingRow>,
    transform_file_id: i64,
    child_file_ids: &[i64],
    parent: EntityId,
    depth: usize,
) -> Result<(), String> {
    let child_depth = depth
        .checked_add(1)
        .ok_or_else(|| "Imported hierarchy depth overflow.".to_owned())?;

    for child_file_id in child_file_ids.iter().rev() {
        pending.push(PendingRow {
            transform_file_id: *child_file_id,
            expected_parent_file_id: transform_file_id,
            parent: Some(parent),
            depth: child_depth,
        });
    }

    Ok(())
}
