//! OpenUnity display-only filtering for discovered project entries.

use openunity_editor_core::project::{ProjectEntry, ProjectEntryKind};

pub(super) fn entry_is_visible(entry: &ProjectEntry) -> bool {
    if entry.kind != ProjectEntryKind::File {
        return true;
    }

    let Some(file_name) = entry.relative_path.file_name() else {
        return true;
    };

    !file_name.as_encoded_bytes().ends_with(b".meta")
}

pub(super) fn visible_indices(entries: &[ProjectEntry]) -> Vec<usize> {
    entries
        .iter()
        .enumerate()
        .filter_map(|(index, entry)| entry_is_visible(entry).then_some(index))
        .collect()
}
