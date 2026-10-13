//! Deterministic package traversal using shared project-wide content budgets.

mod entries;
mod listing;

use std::path::{Path, PathBuf};

use super::super::super::ProjectEntry;
use super::super::super::ProjectPackageDiagnosticKind;
use super::super::super::packages::PackageSourceDescriptor;
use super::super::PackageContent;
use super::budget::{PackageContentBudget, PackageContentState};
use listing::Listing;

pub(in crate::project) fn discover_package_content<'root>(
    descriptor: &'root PackageSourceDescriptor,
    project_root: &'root Path,
    budget: &mut PackageContentBudget,
) -> PackageContent {
    let state = budget.begin_source(&descriptor.name);
    let mut discovery = Discovery::new(descriptor, project_root, state);

    discovery.walk();
    discovery.into_content()
}

struct Discovery<'root, 'budget> {
    source_root: &'root Path,
    project_root: &'root Path,
    state: PackageContentState<'budget>,
    entries: Vec<ProjectEntry>,
}

impl<'root, 'budget> Discovery<'root, 'budget> {
    fn new(
        descriptor: &'root PackageSourceDescriptor,
        project_root: &'root Path,
        state: PackageContentState<'budget>,
    ) -> Self {
        Self {
            source_root: &descriptor.source_root,
            project_root,
            state,
            entries: Vec::new(),
        }
    }

    fn walk(&mut self) {
        if let Some(reason) = self.state.stop_reason() {
            self.report_global_stop(reason);
            return;
        }

        let mut pending = vec![(PathBuf::new(), 0usize)];
        while let Some((relative_directory, depth)) = pending.pop() {
            let children = match self.read_children(&relative_directory) {
                Listing::Complete(children) => children,
                Listing::Skipped => continue,
                Listing::Exhausted => break,
            };

            let directories = self.record_children(children, &relative_directory, depth);
            if self.state.stop_reason().is_some() {
                break;
            }
            pending.extend(directories.into_iter().rev());
        }
    }

    fn report_global_stop(&mut self, reason: ProjectPackageDiagnosticKind) {
        self.state.diagnose(reason, None);
    }

    fn push_diagnostic(&mut self, kind: ProjectPackageDiagnosticKind, path: &Path) {
        self.state.diagnose(kind, Some(path));
    }

    fn into_content(self) -> PackageContent {
        let mut entries = self.entries;
        entries.sort_by(|left, right| left.relative_path.cmp(&right.relative_path));
        PackageContent::new(entries, self.state.into_diagnostics())
    }
}
