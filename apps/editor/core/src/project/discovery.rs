//! Bounded deterministic Assets traversal with private per-discovery state.

use std::path::{Path, PathBuf};

use super::{
    MAX_PROJECT_DEPTH, MAX_PROJECT_ENTRIES, ProjectDiagnostic, ProjectDiagnosticKind, ProjectEntry,
    ProjectEntryKind,
};

mod entries;
mod listing;

use listing::Listing;

struct Discovery<'a> {
    root: &'a Path,
    assets: PathBuf,
    entry_limit: usize,
    depth_limit: usize,
    inspected: usize,
    entries: Vec<ProjectEntry>,
    diagnostics: Vec<ProjectDiagnostic>,
}

pub(super) fn discover_assets(root: &Path) -> (Vec<ProjectEntry>, Vec<ProjectDiagnostic>) {
    discover_assets_with_limits(root, MAX_PROJECT_ENTRIES, MAX_PROJECT_DEPTH)
}

pub(super) fn discover_assets_with_limits(
    root: &Path,
    entry_limit: usize,
    depth_limit: usize,
) -> (Vec<ProjectEntry>, Vec<ProjectDiagnostic>) {
    let mut discovery = Discovery {
        root,
        assets: root.join("Assets"),
        entry_limit,
        depth_limit,
        inspected: 0,
        entries: Vec::new(),
        diagnostics: Vec::new(),
    };
    discovery.walk();
    discovery
        .entries
        .sort_by(|left, right| left.relative_path.cmp(&right.relative_path));
    discovery.diagnostics.sort_by(|left, right| {
        (&left.relative_path, left.kind).cmp(&(&right.relative_path, right.kind))
    });
    (discovery.entries, discovery.diagnostics)
}

impl Discovery<'_> {
    fn walk(&mut self) {
        let mut pending = vec![(PathBuf::new(), 0usize)];
        while let Some((relative_directory, depth)) = pending.pop() {
            let children = match self.read_children(&relative_directory) {
                Listing::Complete(children) => children,
                Listing::Skipped => continue,
                Listing::Exhausted => break,
            };
            let directories = self.record_children(children, &relative_directory, depth);
            pending.extend(directories.into_iter().rev());
        }
    }
}

fn diagnostic(
    path: &Path,
    kind: ProjectDiagnosticKind,
    message: impl Into<String>,
) -> ProjectDiagnostic {
    ProjectDiagnostic {
        relative_path: path.to_path_buf(),
        kind,
        message: message.into(),
    }
}
