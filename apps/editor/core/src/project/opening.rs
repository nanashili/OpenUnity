//! Read-only opening, required path checks and bounded version parsing.

mod package_sources;

use std::fs::{self, File};
use std::io::Read;
use std::path::{Component, Path, PathBuf};

use super::discovery::discover_assets;
use super::index::{ProjectIndex, ProjectSnapshotId};
use super::packages::discover_embedded_packages;
use super::{MAX_PROJECT_VERSION_BYTES, Project, ProjectDiagnostic, ProjectEntry, ProjectError};
use package_sources::discover_package_sources;

impl Project {
    /// Opens an existing source project without writing or executing its contents.
    /// Required paths and all supplied root components must be real, nonsymlink
    /// paths. Unsupported asset entries are omitted and diagnosed. Concurrent
    /// hostile filesystem mutation is outside this reader's guarantee.
    pub fn open(path: impl AsRef<Path>) -> Result<Self, ProjectError> {
        let root = resolve_project_root(path.as_ref())?;
        let assets_root = required_path(&root, Path::new("Assets"), true)?;
        let editor_version = read_project_version(&root)?;
        let (entries, diagnostics) = discover_assets(&root);
        let index = build_project_index(&root, &entries)?;
        let package_catalog = discover_embedded_packages(&root);
        let (packages, package_diagnostics) =
            discover_package_sources(&root, entries.len(), package_catalog);

        Ok(Self {
            root,
            editor_version,
            assets_root,
            entries,
            diagnostics,
            index,
            snapshot_id: ProjectSnapshotId::new(),
            packages,
            package_diagnostics,
        })
    }

    /// Canonical source root, protected against editor writes.
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Declared version token, without a compatibility claim.
    pub fn editor_version(&self) -> &str {
        &self.editor_version
    }

    /// Deterministically sorted, Assets-relative entries.
    pub fn entries(&self) -> &[ProjectEntry] {
        &self.entries
    }

    /// Rejected entries and bounded-traversal diagnostics.
    pub fn diagnostics(&self) -> &[ProjectDiagnostic] {
        &self.diagnostics
    }
}

fn resolve_project_root(supplied: &Path) -> Result<PathBuf, ProjectError> {
    if supplied.as_os_str().is_empty()
        || supplied
            .components()
            .any(|part| matches!(part, Component::ParentDir))
    {
        return Err(error(
            supplied,
            "an explicit root without parent traversal is required",
        ));
    }

    let absolute = if supplied.is_absolute() {
        supplied.to_path_buf()
    } else {
        std::env::current_dir()
            .map_err(|e| error(supplied, e.to_string()))?
            .join(supplied)
    };

    reject_symlink_components(&absolute)?;

    let root = fs::canonicalize(&absolute).map_err(|e| error(&absolute, e.to_string()))?;
    if !root.is_dir() {
        return Err(error(&root, "the project root must be a directory"));
    }

    Ok(root)
}

fn reject_symlink_components(absolute: &Path) -> Result<(), ProjectError> {
    let mut component_path = PathBuf::new();
    for component in absolute.components() {
        component_path.push(component.as_os_str());
        let metadata = fs::symlink_metadata(&component_path)
            .map_err(|e| error(&component_path, e.to_string()))?;
        if metadata.file_type().is_symlink() {
            return Err(error(
                &component_path,
                "symlink root components are unsupported",
            ));
        }
    }

    Ok(())
}

fn read_project_version(root: &Path) -> Result<String, ProjectError> {
    required_path(root, Path::new("ProjectSettings"), true)?;

    let version_path = required_path(root, Path::new("ProjectSettings/ProjectVersion.txt"), false)?;
    read_version(&version_path)
}

fn build_project_index(
    root: &Path,
    entries: &[ProjectEntry],
) -> Result<ProjectIndex, ProjectError> {
    ProjectIndex::build(entries).map_err(|entry_index| {
        let entry_path = root
            .join("Assets")
            .join(&entries[entry_index].relative_path);
        error(
            &entry_path,
            "the discovered entry has no parent directory in the project snapshot",
        )
    })
}

fn error(path: &Path, message: impl Into<String>) -> ProjectError {
    ProjectError {
        path: path.to_path_buf(),
        message: message.into(),
    }
}

fn required_path(root: &Path, relative: &Path, directory: bool) -> Result<PathBuf, ProjectError> {
    let path = root.join(relative);
    let metadata = fs::symlink_metadata(&path).map_err(|e| error(&path, e.to_string()))?;
    if metadata.file_type().is_symlink() {
        return Err(error(&path, "required project paths cannot be symlinks"));
    }
    if (directory && !metadata.is_dir()) || (!directory && !metadata.is_file()) {
        return Err(error(
            &path,
            if directory {
                "a directory is required"
            } else {
                "a regular file is required"
            },
        ));
    }
    let canonical = fs::canonicalize(&path).map_err(|e| error(&path, e.to_string()))?;
    if !canonical.starts_with(root) {
        return Err(error(&path, "required path is outside the project root"));
    }
    Ok(canonical)
}

fn read_version(path: &Path) -> Result<String, ProjectError> {
    let file = File::open(path).map_err(|e| error(path, e.to_string()))?;
    let mut bytes = Vec::new();
    file.take(MAX_PROJECT_VERSION_BYTES as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| error(path, e.to_string()))?;
    if bytes.len() > MAX_PROJECT_VERSION_BYTES {
        return Err(error(path, "ProjectVersion.txt exceeds the 64 KiB limit"));
    }
    let text = std::str::from_utf8(&bytes)
        .map_err(|_| error(path, "ProjectVersion.txt must be UTF-8"))?
        .trim_start_matches('\u{feff}');
    let mut version = None;
    for line in text.lines() {
        if let Some((key, value)) = line.split_once(':')
            && key == "m_EditorVersion"
        {
            if version.is_some() {
                return Err(error(path, "duplicate m_EditorVersion field"));
            }
            let value = value.trim();
            // The supported header carries a compact printable version token.
            // This records the declaration, without claiming support for that editor.
            if value.is_empty()
                || value.len() > 128
                || !value
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'-' | b'_'))
            {
                return Err(error(path, "unsupported m_EditorVersion string"));
            }
            version = Some(value.to_owned());
        }
    }
    version.ok_or_else(|| error(path, "missing m_EditorVersion field"))
}
