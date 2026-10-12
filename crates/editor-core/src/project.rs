//! Read-only discovery of a source project; no import or Unity compatibility is implied.

use std::error::Error;
use std::fmt;
use std::fs::{self, File};
use std::io::Read;
use std::path::{Component, Path, PathBuf};

/// Maximum inspected entries, including rejected entries.
pub const MAX_PROJECT_ENTRIES: usize = 100_000;
/// Maximum Assets traversal depth.
pub const MAX_PROJECT_DEPTH: usize = 256;
/// Maximum ProjectVersion.txt input size.
pub const MAX_PROJECT_VERSION_BYTES: usize = 64 * 1024;

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
/// Supported filesystem entry kinds; contents are never imported.
pub enum ProjectEntryKind {
    /// A real directory.
    Directory,
    /// A regular file.
    File,
}

#[derive(Clone, Debug, Eq, PartialEq)]
/// One discovered Assets-relative entry.
pub struct ProjectEntry {
    /// Relative to Assets, never an absolute path or a parent traversal.
    pub relative_path: PathBuf,
    /// Filesystem kind.
    pub kind: ProjectEntryKind,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
/// Why an entry could not be included.
pub enum ProjectDiagnosticKind {
    /// Symbolic links are not followed.
    Symlink,
    /// The resolved path escaped the project root.
    OutsideRoot,
    /// Filesystem access failed.
    Inaccessible,
    /// The entry is neither a directory nor a regular file.
    UnsupportedEntry,
    /// Traversal reached its depth bound.
    DepthLimit,
    /// Traversal reached its entry bound.
    EntryLimit,
}

#[derive(Clone, Debug, Eq, PartialEq)]
/// A nonfatal discovery failure.
pub struct ProjectDiagnostic {
    /// Relative to Assets; the empty path identifies Assets itself.
    pub relative_path: PathBuf,
    /// Failure classification.
    pub kind: ProjectDiagnosticKind,
    /// User-facing explanation.
    pub message: String,
}

#[derive(Debug)]
/// Failure to open a required project path or version header.
pub struct ProjectError {
    /// Path involved in the operation.
    pub path: PathBuf,
    /// User-facing explanation.
    pub message: String,
}

impl fmt::Display for ProjectError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Cannot open project at {}: {}",
            self.path.display(),
            self.message
        )
    }
}

impl Error for ProjectError {}

#[derive(Debug)]
/// A read-only snapshot of project metadata and Assets entry paths.
pub struct Project {
    root: PathBuf,
    editor_version: String,
    entries: Vec<ProjectEntry>,
    diagnostics: Vec<ProjectDiagnostic>,
}

impl Project {
    /// Opens an existing source project without writing or executing its contents.
    /// Required paths and all supplied root components must be real, nonsymlink
    /// paths. Unsupported asset entries are omitted and diagnosed. Concurrent
    /// hostile filesystem mutation is outside this reader's guarantee.
    pub fn open(path: impl AsRef<Path>) -> Result<Self, ProjectError> {
        let supplied = path.as_ref();
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
        let root = fs::canonicalize(&absolute).map_err(|e| error(&absolute, e.to_string()))?;
        if !root.is_dir() {
            return Err(error(&root, "the project root must be a directory"));
        }
        required_path(&root, Path::new("Assets"), true)?;
        required_path(&root, Path::new("ProjectSettings"), true)?;
        let version_path = required_path(
            &root,
            Path::new("ProjectSettings/ProjectVersion.txt"),
            false,
        )?;
        let editor_version = read_version(&version_path)?;
        let (entries, diagnostics) = discover_assets(&root);
        Ok(Self {
            root,
            editor_version,
            entries,
            diagnostics,
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

fn discover_assets(root: &Path) -> (Vec<ProjectEntry>, Vec<ProjectDiagnostic>) {
    discover_assets_with_limits(root, MAX_PROJECT_ENTRIES, MAX_PROJECT_DEPTH)
}

fn discover_assets_with_limits(
    root: &Path,
    entry_limit: usize,
    depth_limit: usize,
) -> (Vec<ProjectEntry>, Vec<ProjectDiagnostic>) {
    let assets = root.join("Assets");
    let mut entries = Vec::new();
    let mut diagnostics = Vec::new();
    let mut inspected = 0usize;
    let mut pending = vec![(PathBuf::new(), 0usize)];
    while let Some((relative_directory, depth)) = pending.pop() {
        let directory = assets.join(&relative_directory);
        let read_dir = match fs::read_dir(&directory) {
            Ok(read_dir) => read_dir,
            Err(e) => {
                diagnostics.push(diagnostic(
                    &relative_directory,
                    ProjectDiagnosticKind::Inaccessible,
                    e.to_string(),
                ));
                continue;
            }
        };
        // If a directory exceeds the remaining capacity, omit its entire listing.
        // This keeps both memory and results independent of filesystem iteration order.
        let remaining = entry_limit.saturating_sub(inspected);
        let mut children = Vec::new();
        let mut overflow = false;
        let mut inaccessible = false;
        for child in read_dir {
            if children.len() == remaining {
                overflow = true;
                break;
            }
            match child {
                Ok(child) => children.push(child),
                Err(e) => {
                    diagnostics.push(diagnostic(
                        &relative_directory,
                        ProjectDiagnosticKind::Inaccessible,
                        e.to_string(),
                    ));
                    // An unreadable directory iterator can repeatedly fail. Stop here.
                    inaccessible = true;
                    break;
                }
            }
        }
        inspected += children.len();
        if overflow {
            diagnostics.push(diagnostic(
                &relative_directory,
                ProjectDiagnosticKind::EntryLimit,
                "asset discovery reached its entry limit",
            ));
            break;
        }
        if inaccessible {
            // A partial directory listing depends on enumeration order.
            continue;
        }
        children.sort_by_key(|child| child.file_name());
        let mut directories = Vec::new();
        for child in children {
            let relative = relative_directory.join(child.file_name());
            let metadata = match fs::symlink_metadata(child.path()) {
                Ok(metadata) => metadata,
                Err(e) => {
                    diagnostics.push(diagnostic(
                        &relative,
                        ProjectDiagnosticKind::Inaccessible,
                        e.to_string(),
                    ));
                    continue;
                }
            };
            if metadata.file_type().is_symlink() {
                diagnostics.push(diagnostic(
                    &relative,
                    ProjectDiagnosticKind::Symlink,
                    "symlink asset entries are unsupported",
                ));
                continue;
            }
            let canonical = match fs::canonicalize(child.path()) {
                Ok(path) => path,
                Err(e) => {
                    diagnostics.push(diagnostic(
                        &relative,
                        ProjectDiagnosticKind::Inaccessible,
                        e.to_string(),
                    ));
                    continue;
                }
            };
            if !canonical.starts_with(&assets) || !canonical.starts_with(root) {
                diagnostics.push(diagnostic(
                    &relative,
                    ProjectDiagnosticKind::OutsideRoot,
                    "asset path is outside Assets",
                ));
                continue;
            }
            let kind = if metadata.is_dir() {
                ProjectEntryKind::Directory
            } else if metadata.is_file() {
                ProjectEntryKind::File
            } else {
                diagnostics.push(diagnostic(
                    &relative,
                    ProjectDiagnosticKind::UnsupportedEntry,
                    "only regular files and directories are supported",
                ));
                continue;
            };
            entries.push(ProjectEntry {
                relative_path: relative.clone(),
                kind,
            });
            if kind == ProjectEntryKind::Directory {
                if depth + 1 >= depth_limit {
                    diagnostics.push(diagnostic(
                        &relative,
                        ProjectDiagnosticKind::DepthLimit,
                        "asset discovery reached its directory depth limit",
                    ));
                } else {
                    directories.push((relative, depth + 1));
                }
            }
        }
        pending.extend(directories.into_iter().rev());
    }
    entries.sort_by(|left, right| left.relative_path.cmp(&right.relative_path));
    diagnostics.sort_by(|left, right| {
        (&left.relative_path, left.kind).cmp(&(&right.relative_path, right.kind))
    });
    (entries, diagnostics)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT_FIXTURE: AtomicU64 = AtomicU64::new(1);

    struct Fixture(PathBuf);

    impl Fixture {
        fn new() -> Self {
            let base = std::env::temp_dir().canonicalize().unwrap();
            let path = base.join(format!(
                "openunity-project-{}-{}",
                std::process::id(),
                NEXT_FIXTURE.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir(&path).unwrap();
            fs::create_dir(path.join("Assets")).unwrap();
            fs::create_dir(path.join("ProjectSettings")).unwrap();
            fs::write(path.join("ProjectSettings/ProjectVersion.txt"), "m_EditorVersion: 6000.0.23f1\nm_EditorVersionWithRevision: 6000.0.23f1 (independent-fixture)\n").unwrap();
            Self(path)
        }
    }

    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn opens_read_only_without_library_and_sorts_assets_relative_paths() {
        let fixture = Fixture::new();
        fs::create_dir(fixture.0.join("Assets/Zebra")).unwrap();
        fs::write(fixture.0.join("Assets/Zebra/b.txt"), "independent asset").unwrap();
        fs::write(fixture.0.join("Assets/a.txt"), "original bytes").unwrap();
        let project = Project::open(&fixture.0).unwrap();
        assert_eq!(project.root(), fixture.0);
        assert_eq!(project.editor_version(), "6000.0.23f1");
        assert_eq!(
            project
                .entries()
                .iter()
                .map(|entry| (entry.relative_path.as_path(), entry.kind))
                .collect::<Vec<_>>(),
            vec![
                (Path::new("Zebra"), ProjectEntryKind::Directory),
                (Path::new("Zebra/b.txt"), ProjectEntryKind::File),
                (Path::new("a.txt"), ProjectEntryKind::File),
            ]
        );
        assert!(project.diagnostics().is_empty());
        assert_eq!(
            fs::read(fixture.0.join("Assets/a.txt")).unwrap(),
            b"original bytes"
        );
        assert!(!fixture.0.join("Library").exists());
        assert_eq!(fs::read_dir(&fixture.0).unwrap().count(), 2);
    }

    #[test]
    fn missing_required_paths_and_parent_traversal_are_errors() {
        let fixture = Fixture::new();
        assert!(Project::open(fixture.0.join("Assets/..")).is_err());
        fs::remove_dir(fixture.0.join("Assets")).unwrap();
        assert!(
            Project::open(&fixture.0)
                .unwrap_err()
                .path
                .ends_with("Assets")
        );
    }

    #[test]
    fn version_header_is_bounded_and_validated() {
        let fixture = Fixture::new();
        let path = fixture.0.join("ProjectSettings/ProjectVersion.txt");
        for invalid in [
            b"other: value".as_slice(),
            b"m_EditorVersion: \n",
            b"m_EditorVersion: a\nm_EditorVersion: b\n",
            b"m_EditorVersion: bad version\n",
            b"\xff",
        ] {
            fs::write(&path, invalid).unwrap();
            assert!(Project::open(&fixture.0).is_err(), "accepted {invalid:?}");
        }
        fs::write(&path, vec![b'x'; MAX_PROJECT_VERSION_BYTES + 1]).unwrap();
        assert!(
            Project::open(&fixture.0)
                .unwrap_err()
                .message
                .contains("64 KiB")
        );
        fs::write(&path, "\u{feff}m_EditorVersion: 2022.3.0f1\r\n").unwrap();
        assert_eq!(
            Project::open(&fixture.0).unwrap().editor_version(),
            "2022.3.0f1"
        );
    }

    #[test]
    fn discovery_limits_are_deterministic_and_diagnosed() {
        let fixture = Fixture::new();
        fs::create_dir_all(fixture.0.join("Assets/one/two/three")).unwrap();
        let (entries, diagnostics) = discover_assets_with_limits(&fixture.0, 10, 2);
        assert_eq!(entries.len(), 2);
        assert_eq!(diagnostics[0].kind, ProjectDiagnosticKind::DepthLimit);
        let (entries, diagnostics) = discover_assets_with_limits(&fixture.0, 1, 10);
        assert_eq!(entries.len(), 1);
        assert_eq!(diagnostics[0].kind, ProjectDiagnosticKind::EntryLimit);
        fs::write(fixture.0.join("Assets/a.txt"), "a").unwrap();
        let (entries, diagnostics) = discover_assets_with_limits(&fixture.0, 1, 10);
        assert!(entries.is_empty());
        assert_eq!(diagnostics[0].relative_path, Path::new(""));
        assert_eq!(diagnostics[0].kind, ProjectDiagnosticKind::EntryLimit);
    }

    #[cfg(unix)]
    #[test]
    fn rejects_symlink_roots_required_paths_and_asset_links() {
        use std::os::unix::fs::symlink;
        let fixture = Fixture::new();
        let outside = Fixture::new();
        fs::write(outside.0.join("secret.txt"), "outside fixture").unwrap();
        symlink(&outside.0, fixture.0.join("Assets/escape")).unwrap();
        symlink("missing", fixture.0.join("Assets/dangling")).unwrap();
        let project = Project::open(&fixture.0).unwrap();
        assert!(project.entries().is_empty());
        assert_eq!(project.diagnostics().len(), 2);
        assert!(
            project
                .diagnostics()
                .iter()
                .all(|d| d.kind == ProjectDiagnosticKind::Symlink)
        );
        symlink(&outside.0, fixture.0.join("linked-root")).unwrap();
        assert!(Project::open(fixture.0.join("linked-root")).is_err());
        fs::remove_file(fixture.0.join("ProjectSettings/ProjectVersion.txt")).unwrap();
        symlink(
            outside.0.join("ProjectSettings/ProjectVersion.txt"),
            fixture.0.join("ProjectSettings/ProjectVersion.txt"),
        )
        .unwrap();
        assert!(Project::open(&fixture.0).is_err());
        fs::remove_dir_all(fixture.0.join("Assets")).unwrap();
        symlink(outside.0.join("Assets"), fixture.0.join("Assets")).unwrap();
        assert!(Project::open(&fixture.0).is_err());
    }

    #[cfg(unix)]
    #[test]
    fn rejected_entries_also_consume_the_global_discovery_budget() {
        use std::os::unix::fs::symlink;
        let fixture = Fixture::new();
        fs::create_dir(fixture.0.join("Assets/a")).unwrap();
        fs::create_dir(fixture.0.join("Assets/b")).unwrap();
        symlink("missing", fixture.0.join("Assets/a/x")).unwrap();
        symlink("missing", fixture.0.join("Assets/a/y")).unwrap();
        fs::write(fixture.0.join("Assets/b/hidden-by-limit"), "asset").unwrap();
        let (entries, diagnostics) = discover_assets_with_limits(&fixture.0, 4, 10);
        assert_eq!(entries.len(), 2);
        assert_eq!(diagnostics.len(), 3);
        assert_eq!(diagnostics[2].relative_path, Path::new("b"));
        assert_eq!(diagnostics[2].kind, ProjectDiagnosticKind::EntryLimit);
    }

    #[cfg(unix)]
    #[test]
    fn unsupported_socket_is_diagnosed_without_opening_it() {
        let fixture = Fixture::new();
        let _socket =
            std::os::unix::net::UnixListener::bind(fixture.0.join("Assets/socket")).unwrap();
        let project = Project::open(&fixture.0).unwrap();
        assert!(project.entries().is_empty());
        assert_eq!(
            project.diagnostics()[0].kind,
            ProjectDiagnosticKind::UnsupportedEntry
        );
    }

    #[cfg(unix)]
    #[test]
    fn inaccessible_directory_produces_a_diagnostic_when_permissions_are_enforced() {
        use std::os::unix::fs::PermissionsExt;
        let fixture = Fixture::new();
        let locked = fixture.0.join("Assets/locked");
        fs::create_dir(&locked).unwrap();
        fs::set_permissions(&locked, fs::Permissions::from_mode(0o0)).unwrap();
        let inaccessible = fs::read_dir(&locked).is_err();
        let project = Project::open(&fixture.0).unwrap();
        fs::set_permissions(&locked, fs::Permissions::from_mode(0o700)).unwrap();
        if inaccessible {
            assert!(
                project
                    .diagnostics()
                    .iter()
                    .any(|d| d.relative_path == Path::new("locked")
                        && d.kind == ProjectDiagnosticKind::Inaccessible)
            );
        }
    }
}
