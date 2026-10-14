//! Build an authored-order projection without repeated document queries.

mod index;
mod traversal;

use openunity_editor_core::imported_scene::ImportedDocument;

use super::super::super::hierarchy_browser::Row;

pub(super) fn build(document: &ImportedDocument) -> Result<Vec<Row>, String> {
    let index = index::ProjectionIndex::new(document)?;
    let rows = traversal::preorder_rows(document, &index)?;

    index.ensure_complete(document, rows.len())?;
    Ok(rows)
}
