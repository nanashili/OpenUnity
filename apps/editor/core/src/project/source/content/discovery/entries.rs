//! Inspect package children without following links or escaping source roots.

use std::fs::{self, DirEntry};
use std::path::{Path, PathBuf};

use crate::project::{
    MAX_PROJECT_DEPTH, ProjectEntry, ProjectEntryKind, ProjectPackageDiagnosticKind,
};

use super::Discovery;

impl Discovery<'_, '_> {
    pub(super) fn record_children(
        &mut self,
        children: Vec<DirEntry>,
        relative_directory: &Path,
        depth: usize,
    ) -> Vec<(PathBuf, usize)> {
        let mut directories = Vec::new();

        for child in children {
            if self.state.stop_reason().is_some() {
                break;
            }

            let Some((relative_path, kind)) = self.inspect_entry(child, relative_directory) else {
                continue;
            };
            if !self.state.retain_entry(&relative_path) {
                break;
            }

            self.entries.push(ProjectEntry {
                relative_path: relative_path.clone(),
                kind,
            });

            if kind == ProjectEntryKind::Directory {
                if depth + 1 >= MAX_PROJECT_DEPTH {
                    self.push_diagnostic(ProjectPackageDiagnosticKind::DepthLimit, &relative_path);
                } else {
                    directories.push((relative_path, depth + 1));
                }
            }
        }

        directories
    }

    fn inspect_entry(
        &mut self,
        child: DirEntry,
        relative_directory: &Path,
    ) -> Option<(PathBuf, ProjectEntryKind)> {
        let relative_path = relative_directory.join(child.file_name());
        let path_bytes = relative_path.as_os_str().as_encoded_bytes().len();
        if path_bytes > 4_096 {
            self.push_diagnostic(
                ProjectPackageDiagnosticKind::PathLengthLimit,
                &relative_path,
            );
            return None;
        }

        let metadata = match fs::symlink_metadata(child.path()) {
            Ok(metadata) => metadata,
            Err(_) => {
                self.push_diagnostic(ProjectPackageDiagnosticKind::Inaccessible, &relative_path);
                return None;
            }
        };
        if metadata.file_type().is_symlink() {
            self.push_diagnostic(ProjectPackageDiagnosticKind::Symlink, &relative_path);
            return None;
        }

        let canonical = match fs::canonicalize(child.path()) {
            Ok(path) => path,
            Err(_) => {
                self.push_diagnostic(ProjectPackageDiagnosticKind::Inaccessible, &relative_path);
                return None;
            }
        };
        if !canonical.starts_with(self.source_root) || !canonical.starts_with(self.project_root) {
            self.push_diagnostic(ProjectPackageDiagnosticKind::OutsideSource, &relative_path);
            return None;
        }

        let kind = if metadata.is_dir() {
            ProjectEntryKind::Directory
        } else if metadata.is_file() {
            ProjectEntryKind::File
        } else {
            self.push_diagnostic(
                ProjectPackageDiagnosticKind::UnsupportedEntry,
                &relative_path,
            );
            return None;
        };

        Some((relative_path, kind))
    }
}
