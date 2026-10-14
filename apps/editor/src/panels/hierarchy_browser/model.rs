use openunity_editor_core::{hierarchy::HierarchyRevision, scene::EntityId};
use std::collections::{BTreeMap, HashSet};

/// Document-owned hierarchy data passed by the authoring adapters.
pub(crate) struct Row {
    pub id: EntityId,
    pub parent: Option<EntityId>,
    pub name: String,
    pub depth: usize,
    pub active: bool,
    pub sibling_position: Option<(usize, usize)>,
    pub root_position: Option<(usize, usize)>,
}

pub(crate) enum BrowserAction {
    Select(EntityId, bool),
    SetSiblingIndex(EntityId, usize),
}

pub(super) struct CachedRow {
    pub(super) id: EntityId,
    pub(super) parent: Option<EntityId>,
    pub(super) name: String,
    pub(super) lowercase_name: String,
    pub(super) depth: usize,
    pub(super) active: bool,
    pub(super) has_children: bool,
    pub(super) sibling_position: Option<(usize, usize)>,
    pub(super) root_position: Option<(usize, usize)>,
}

pub(super) struct CachedForest {
    pub(super) revision: HierarchyRevision,
    pub(super) rows: Vec<CachedRow>,
    pub(super) sibling_groups: BTreeMap<Option<EntityId>, Vec<EntityId>>,
    pub(super) parent_indices: Vec<Option<usize>>,
    pub(super) branches: HashSet<EntityId>,
}

struct PreorderTopology {
    parent_indices: Vec<Option<usize>>,
    branches: HashSet<EntityId>,
    has_children: Vec<bool>,
}

impl CachedForest {
    pub(super) fn from_rows(revision: HierarchyRevision, rows: Vec<Row>) -> Result<Self, String> {
        let index_by_id = index_rows_by_id(&rows)?;
        let topology = PreorderTopology::validate(&rows, &index_by_id)?;
        let mut sibling_groups = BTreeMap::new();
        for row in &rows {
            sibling_groups
                .entry(row.parent)
                .or_insert_with(Vec::new)
                .push(row.id);
        }
        let cached_rows = cache_rows(rows, &topology.has_children);

        Ok(Self {
            revision,
            rows: cached_rows,
            sibling_groups,
            parent_indices: topology.parent_indices,
            branches: topology.branches,
        })
    }
}

fn index_rows_by_id(rows: &[Row]) -> Result<BTreeMap<EntityId, usize>, String> {
    let mut index_by_id = BTreeMap::new();
    for (index, row) in rows.iter().enumerate() {
        if index_by_id.insert(row.id, index).is_some() {
            return Err(format!(
                "Hierarchy source contains duplicate editor ID {}.",
                row.id.0
            ));
        }
    }

    Ok(index_by_id)
}

impl PreorderTopology {
    fn validate(rows: &[Row], index_by_id: &BTreeMap<EntityId, usize>) -> Result<Self, String> {
        let mut topology = Self {
            parent_indices: Vec::with_capacity(rows.len()),
            branches: HashSet::new(),
            has_children: vec![false; rows.len()],
        };
        let mut ancestry = Vec::new();

        for (index, row) in rows.iter().enumerate() {
            validate_depth(row, &mut ancestry)?;
            let parent = parent_for_row(row, index, index_by_id, &ancestry)?;
            topology.record_parent(parent);
            ancestry.push(index);
        }

        Ok(topology)
    }

    fn record_parent(&mut self, parent: Option<(usize, EntityId)>) {
        if let Some((parent_index, parent_id)) = parent {
            self.has_children[parent_index] = true;
            self.branches.insert(parent_id);
        }
        self.parent_indices
            .push(parent.map(|(parent_index, _)| parent_index));
    }
}

fn validate_depth(row: &Row, ancestry: &mut Vec<usize>) -> Result<(), String> {
    if row.depth > ancestry.len() {
        return Err(format!(
            "Hierarchy row {} skips a preorder depth.",
            row.id.0
        ));
    }

    ancestry.truncate(row.depth);
    Ok(())
}

fn parent_for_row(
    row: &Row,
    row_index: usize,
    index_by_id: &BTreeMap<EntityId, usize>,
    ancestry: &[usize],
) -> Result<Option<(usize, EntityId)>, String> {
    let Some(parent_id) = row.parent else {
        if row.depth == 0 {
            return Ok(None);
        }
        return Err(format!(
            "Hierarchy row {} has no parent at depth {}.",
            row.id.0, row.depth
        ));
    };
    let Some(parent_index) = index_by_id.get(&parent_id).copied() else {
        return Err(format!(
            "Hierarchy row {} references missing parent {}.",
            row.id.0, parent_id.0
        ));
    };

    if parent_index >= row_index || row.depth == 0 {
        return Err(format!(
            "Hierarchy row {} has an invalid parent order.",
            row.id.0
        ));
    }
    if ancestry.get(row.depth - 1).copied() != Some(parent_index) {
        return Err(format!(
            "Hierarchy row {} is not nested beneath its declared parent.",
            row.id.0
        ));
    }

    Ok(Some((parent_index, parent_id)))
}

fn cache_rows(rows: Vec<Row>, has_children: &[bool]) -> Vec<CachedRow> {
    rows.into_iter()
        .enumerate()
        .map(|(index, row)| CachedRow {
            lowercase_name: row.name.to_lowercase(),
            id: row.id,
            parent: row.parent,
            name: row.name,
            depth: row.depth,
            active: row.active,
            has_children: has_children[index],
            sibling_position: row.sibling_position,
            root_position: row.root_position,
        })
        .collect()
}
