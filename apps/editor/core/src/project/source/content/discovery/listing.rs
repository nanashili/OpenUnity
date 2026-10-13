//! Read each package directory as a bounded, sorted batch.

use std::fs::{self, DirEntry};
use std::path::Path;

use super::Discovery;
use crate::project::ProjectPackageDiagnosticKind;

impl Discovery<'_, '_> {
    pub(super) fn read_children(&mut self, relative_directory: &Path) -> Listing {
        let directory = self.source_root.join(relative_directory);
        let mut read_dir = match fs::read_dir(&directory) {
            Ok(read_dir) => read_dir,
            Err(_) => {
                self.push_diagnostic(
                    ProjectPackageDiagnosticKind::Inaccessible,
                    relative_directory,
                );
                return Listing::Skipped;
            }
        };

        let mut children = Vec::new();
        loop {
            match read_dir.next() {
                None => {
                    children.sort_by_key(DirEntry::file_name);
                    return Listing::Complete(children);
                }
                Some(Err(_)) => {
                    self.push_diagnostic(
                        ProjectPackageDiagnosticKind::Inaccessible,
                        relative_directory,
                    );
                    return Listing::Skipped;
                }
                Some(Ok(child)) => {
                    if !self.state.can_inspect_next(relative_directory) {
                        return Listing::Exhausted;
                    }

                    self.state.record_inspected_entry();
                    children.push(child);
                }
            }
        }
    }
}

pub(super) enum Listing {
    Complete(Vec<DirEntry>),
    Skipped,
    Exhausted,
}
