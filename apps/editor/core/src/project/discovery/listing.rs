//! Bound each directory listing before accepting any of its entries.

use std::fs::{self, DirEntry};
use std::path::Path;

use super::{Discovery, ProjectDiagnosticKind, diagnostic};

pub(super) enum Listing {
    Complete(Vec<DirEntry>),
    Skipped,
    Exhausted,
}

enum CollectionOutcome {
    Complete,
    Skipped,
    Exhausted,
}

impl Discovery<'_> {
    pub(super) fn read_children(&mut self, relative_directory: &Path) -> Listing {
        let directory = self.assets.join(relative_directory);
        let mut read_dir = match fs::read_dir(&directory) {
            Ok(read_dir) => read_dir,
            Err(e) => {
                self.diagnostics.push(diagnostic(
                    relative_directory,
                    ProjectDiagnosticKind::Inaccessible,
                    e.to_string(),
                ));
                return Listing::Skipped;
            }
        };
        // If a directory exceeds the remaining capacity, omit its entire listing.
        // This keeps both memory and results independent of filesystem iteration order.
        let remaining = self.entry_limit.saturating_sub(self.inspected);
        let mut children = Vec::new();
        let outcome = loop {
            let Some(child) = read_dir.next() else {
                break CollectionOutcome::Complete;
            };
            if children.len() == remaining {
                break CollectionOutcome::Exhausted;
            }
            match child {
                Ok(child) => children.push(child),
                Err(e) => {
                    self.diagnostics.push(diagnostic(
                        relative_directory,
                        ProjectDiagnosticKind::Inaccessible,
                        e.to_string(),
                    ));
                    // An unreadable directory iterator can repeatedly fail. Stop here.
                    break CollectionOutcome::Skipped;
                }
            }
        };
        self.inspected += children.len();
        match outcome {
            CollectionOutcome::Exhausted => {
                self.diagnostics.push(diagnostic(
                    relative_directory,
                    ProjectDiagnosticKind::EntryLimit,
                    "asset discovery reached its entry limit",
                ));
                Listing::Exhausted
            }
            // A partial directory listing depends on enumeration order.
            CollectionOutcome::Skipped => Listing::Skipped,
            CollectionOutcome::Complete => {
                children.sort_by_key(|child| child.file_name());
                Listing::Complete(children)
            }
        }
    }
}
