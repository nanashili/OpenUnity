use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use super::discovery::discover_assets_with_limits;
use super::*;

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
        if let Err(cause) = fs::remove_dir_all(&self.0) {
            let message = format!("failed to remove fixture {}: {cause}", self.0.display());
            if std::thread::panicking() {
                use std::io::Write;
                // Reporting must not replace the initiating assertion panic,
                // including when the diagnostic stream itself is unavailable.
                let _ = writeln!(std::io::stderr().lock(), "{message}");
            } else {
                panic!("{message}");
            }
        }
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
    let _socket = std::os::unix::net::UnixListener::bind(fixture.0.join("Assets/socket")).unwrap();
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
