//! UTF-8 candidate names in deterministic technical-name order.

use std::fs::DirEntry;

use super::super::model::CatalogDiagnostics;
use crate::project::ProjectPackageDiagnosticKind;

pub(super) struct NamedChild {
    pub(super) name: String,
    pub(super) entry: DirEntry,
}

pub(super) fn sort_named_children(
    children: Vec<DirEntry>,
    diagnostics: &mut CatalogDiagnostics,
) -> Vec<NamedChild> {
    let mut named_children = Vec::with_capacity(children.len());

    for entry in children {
        let file_name = entry.file_name();
        let Some(name) = file_name.to_str() else {
            diagnostics.push(ProjectPackageDiagnosticKind::CandidateInvalid, None, None);
            continue;
        };

        named_children.push(NamedChild {
            name: name.to_owned(),
            entry,
        });
    }

    named_children.sort_by(|left, right| left.name.cmp(&right.name));
    named_children
}
