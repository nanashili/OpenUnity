//! Validate confined asset entries and schedule accepted child directories.

use std::fs::{self, DirEntry};
use std::path::{Path, PathBuf};

use super::{Discovery, ProjectDiagnosticKind, ProjectEntry, ProjectEntryKind, diagnostic};

impl Discovery<'_> {
    pub(super) fn record_children(
        &mut self,
        children: Vec<DirEntry>,
        relative_directory: &Path,
        depth: usize,
    ) -> Vec<(PathBuf, usize)> {
        let mut directories = Vec::new();
        for child in children {
            let Some((relative, kind)) = self.inspect_entry(child, relative_directory) else {
                continue;
            };
            self.entries.push(ProjectEntry {
                relative_path: relative.clone(),
                kind,
            });
            if kind == ProjectEntryKind::Directory {
                if depth + 1 >= self.depth_limit {
                    self.diagnostics.push(diagnostic(
                        &relative,
                        ProjectDiagnosticKind::DepthLimit,
                        "asset discovery reached its directory depth limit",
                    ));
                } else {
                    directories.push((relative, depth + 1));
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
        let relative = relative_directory.join(child.file_name());
        let metadata = match fs::symlink_metadata(child.path()) {
            Ok(metadata) => metadata,
            Err(e) => {
                self.diagnostics.push(diagnostic(
                    &relative,
                    ProjectDiagnosticKind::Inaccessible,
                    e.to_string(),
                ));
                return None;
            }
        };
        if metadata.file_type().is_symlink() {
            self.diagnostics.push(diagnostic(
                &relative,
                ProjectDiagnosticKind::Symlink,
                "symlink asset entries are unsupported",
            ));
            return None;
        }
        let canonical = match fs::canonicalize(child.path()) {
            Ok(path) => path,
            Err(e) => {
                self.diagnostics.push(diagnostic(
                    &relative,
                    ProjectDiagnosticKind::Inaccessible,
                    e.to_string(),
                ));
                return None;
            }
        };
        if !canonical.starts_with(&self.assets) || !canonical.starts_with(self.root) {
            self.diagnostics.push(diagnostic(
                &relative,
                ProjectDiagnosticKind::OutsideRoot,
                "asset path is outside Assets",
            ));
            return None;
        }
        let kind = if metadata.is_dir() {
            ProjectEntryKind::Directory
        } else if metadata.is_file() {
            ProjectEntryKind::File
        } else {
            self.diagnostics.push(diagnostic(
                &relative,
                ProjectDiagnosticKind::UnsupportedEntry,
                "only regular files and directories are supported",
            ));
            return None;
        };
        Some((relative, kind))
    }
}
