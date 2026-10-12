//! Public acceptance for bounded metadata discovery and preserved filesystem behavior.

use engine_assets::{
    DuplicateGuid, Guid, MAX_METADATA_BYTES, MetadataDiscoveryError, MetadataDiscoveryLimits,
    MetadataError, discover_metadata_index,
};
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

const GUID_A: &str = "0123456789abcdef0123456789abcdef";
const GUID_B: &str = "abcdefabcdefabcdefabcdefabcdefab";

type MetadataReaderFailureCase = (&'static str, &'static [u8], fn(&MetadataError) -> bool);

struct Fixture(PathBuf);

impl Fixture {
    fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let path = std::env::temp_dir().join(format!(
            "openunity-discovery-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).expect("create isolated fixture");
        Self(fs::canonicalize(path).expect("canonicalize isolated fixture"))
    }

    fn path(&self, relative: impl AsRef<Path>) -> PathBuf {
        self.0.join(relative)
    }

    fn metadata(&self, asset: impl AsRef<Path>, guid: &str) -> PathBuf {
        let asset = self.path(asset);
        if let Some(parent) = asset.parent() {
            fs::create_dir_all(parent).expect("create asset parent");
        }
        fs::write(&asset, b"asset").expect("write asset");
        let metadata = asset.with_extension(format!(
            "{}.meta",
            asset
                .extension()
                .and_then(|value| value.to_str())
                .unwrap_or("")
        ));
        fs::write(&metadata, format!("fileFormatVersion: 2\nguid: {guid}\n"))
            .expect("write metadata");
        metadata
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        match fs::remove_dir_all(&self.0) {
            Ok(()) => {}
            Err(error) if std::thread::panicking() => {
                eprintln!("fixture cleanup failed during unwinding: {error}");
            }
            Err(error) => panic!("fixture cleanup failed: {error}"),
        }
    }
}

fn limits(max_depth: usize, max_entries: usize) -> MetadataDiscoveryLimits {
    MetadataDiscoveryLimits {
        max_depth,
        max_entries,
    }
}

fn valid_index(fixture: &Fixture) -> engine_assets::MetadataIndex {
    discover_metadata_index(&fixture.0, limits(64, 10_000)).expect("valid metadata tree")
}

fn assert_display_contains<T: std::fmt::Debug>(error: &T, expected: &str) {
    let text = format!("{error:?}").to_ascii_lowercase();
    assert!(
        text.contains(expected),
        "{text:?} does not contain {expected:?}"
    );
}

fn source_is<T: Error + 'static>(error: &(dyn Error + 'static)) -> bool {
    let mut current = error.source();
    while let Some(source) = current {
        if source.downcast_ref::<T>().is_some() {
            return true;
        }
        current = source.source();
    }
    false
}

#[test]
fn nested_metadata_is_indexed_with_exact_asset_paths() {
    let fixture = Fixture::new();
    let hero = fixture.metadata("Assets/Nested/hero.cs", GUID_A);
    let texture = fixture.metadata("Assets/texture.png", GUID_B);
    fs::write(fixture.path("Assets/readme.cs"), b"source without metadata").unwrap();

    let index = valid_index(&fixture);
    assert_eq!(index.len(), 2);
    let hero_asset = hero.with_extension("");
    let texture_asset = texture.with_extension("");
    assert_eq!(
        index.asset_path(GUID_A.parse::<Guid>().unwrap()),
        Some(hero_asset.as_path())
    );
    assert_eq!(
        index.asset_path(GUID_B.parse::<Guid>().unwrap()),
        Some(texture_asset.as_path())
    );
}

#[test]
fn duplicate_guid_reports_both_paths_without_partial_index() {
    let fixture = Fixture::new();
    let first = fixture.metadata("A/first.asset", GUID_A).with_extension("");
    let second = fixture
        .metadata("B/second.asset", GUID_A)
        .with_extension("");
    let error = discover_metadata_index(&fixture.0, limits(64, 10_000)).unwrap_err();

    assert!(source_is::<DuplicateGuid>(&error));
    let text = format!("{error:?}");
    assert!(text.contains(&first.display().to_string()));
    assert!(text.contains(&second.display().to_string()));
    assert!(text.contains(GUID_A));
}

#[test]
fn invalid_roots_are_rejected() {
    let fixture = Fixture::new();
    fs::write(fixture.path("file"), b"file").unwrap();
    for root in [
        PathBuf::from("relative"),
        fixture.path(".."),
        fixture.path("file"),
    ] {
        let error = discover_metadata_index(root, limits(64, 10_000)).unwrap_err();
        assert!(matches!(error, MetadataDiscoveryError::InvalidRoot(_)));
    }
    discover_metadata_index(fixture.path("."), limits(64, 10_000))
        .expect("normalized absolute dot root remains accepted");
}

#[cfg(unix)]
#[test]
fn root_and_nested_symlinks_are_rejected() {
    use std::os::unix::fs::symlink;

    let fixture = Fixture::new();
    let real = fixture.path("real");
    fs::create_dir(&real).unwrap();
    let root_link = fixture.path("root-link");
    symlink(&real, &root_link).unwrap();
    let error = discover_metadata_index(root_link, limits(64, 10_000)).unwrap_err();
    assert!(matches!(error, MetadataDiscoveryError::Symlink(_)));

    let nested = Fixture::new();
    symlink(nested.path("Assets"), nested.path("linked-dir")).unwrap();
    let error = discover_metadata_index(&nested.0, limits(64, 10_000)).unwrap_err();
    assert!(matches!(error, MetadataDiscoveryError::Symlink(_)));

    let metadata_link = Fixture::new();
    metadata_link.metadata("Assets/hero.cs", GUID_A);
    fs::remove_file(metadata_link.path("Assets/hero.cs.meta")).unwrap();
    symlink(
        metadata_link.path("Assets/hero.cs"),
        metadata_link.path("Assets/hero.cs.meta"),
    )
    .unwrap();
    let error = discover_metadata_index(&metadata_link.0, limits(64, 10_000)).unwrap_err();
    assert!(matches!(error, MetadataDiscoveryError::Symlink(_)));

    let derived_link = Fixture::new();
    let metadata = derived_link.metadata("Assets/hero.cs", GUID_A);
    fs::remove_file(derived_link.path("Assets/hero.cs")).unwrap();
    fs::write(derived_link.path("target.cs"), b"asset").unwrap();
    symlink(
        derived_link.path("target.cs"),
        derived_link.path("Assets/hero.cs"),
    )
    .unwrap();
    let error = discover_metadata_index(&derived_link.0, limits(64, 10_000)).unwrap_err();
    assert!(matches!(
        error,
        MetadataDiscoveryError::Symlink(path)
            if path == derived_link.path("Assets/hero.cs")
    ));
    assert!(metadata.exists());
}

#[test]
fn exact_depth_and_entry_limits_accept_boundary_values() {
    let depth = Fixture::new();
    fs::create_dir_all(depth.path("one/two")).unwrap();
    discover_metadata_index(&depth.0, limits(2, 10_000)).expect("depth boundary accepted");
    let error = discover_metadata_index(&depth.0, limits(1, 10_000)).unwrap_err();
    assert!(matches!(error, MetadataDiscoveryError::DepthLimit(_)));

    let entries = Fixture::new();
    fs::create_dir(entries.path("nested")).unwrap();
    fs::write(entries.path("nested/file"), b"file").unwrap();
    discover_metadata_index(&entries.0, limits(64, 2)).expect("entry boundary accepted");
    fs::write(entries.path("extra"), b"file").unwrap();
    let error = discover_metadata_index(&entries.0, limits(64, 2)).unwrap_err();
    assert!(matches!(
        error,
        MetadataDiscoveryError::EntryCountLimit { .. }
    ));
}

#[cfg(unix)]
#[test]
fn unsupported_unix_socket_is_rejected_as_a_typed_entry_error() {
    use std::os::unix::net::UnixListener;

    let fixture = Fixture::new();
    let socket = fixture.path("socket");
    let _listener = UnixListener::bind(&socket).unwrap();
    let error = discover_metadata_index(&fixture.0, limits(64, 10_000)).unwrap_err();
    assert!(matches!(
        error,
        MetadataDiscoveryError::UnsupportedEntryType(path) if path == socket
    ));
}

#[test]
fn metadata_reader_failures_are_wrapped_without_reinterpretation() {
    let cases: &[MetadataReaderFailureCase] = &[
        ("malformed.meta", b"fileFormatVersion 2\n", |error| {
            matches!(error, MetadataError::MalformedHeader { .. })
        }),
        ("utf8.meta", &[0xff, 0xfe], |error| {
            matches!(error, MetadataError::InvalidUtf8(_))
        }),
        (
            "missing.meta",
            b"guid: 0123456789abcdef0123456789abcdef\n",
            |error| matches!(error, MetadataError::MissingHeader("fileFormatVersion")),
        ),
        (
            "unsupported.meta",
            b"fileFormatVersion: 3\nguid: 0123456789abcdef0123456789abcdef\n",
            |error| matches!(error, MetadataError::UnsupportedVersion(_)),
        ),
    ];
    for (name, bytes, predicate) in cases {
        let fixture = Fixture::new();
        let path = fixture.path(name);
        fs::write(path.with_extension("asset"), b"asset").unwrap();
        fs::write(&path, bytes).unwrap();
        let error = discover_metadata_index(&fixture.0, limits(64, 10_000)).unwrap_err();
        assert!(source_is::<MetadataError>(&error));
        let text = format!("{error:?}");
        assert!(text.contains(name));
        let source = find_source::<MetadataError>(&error);
        assert!(source.is_some_and(predicate));
    }

    let fixture = Fixture::new();
    let path = fixture.metadata("large.asset", GUID_A);
    let mut bytes = fs::read(&path).unwrap();
    bytes.resize(MAX_METADATA_BYTES + 1, b'x');
    fs::write(&path, bytes).unwrap();
    let error = discover_metadata_index(&fixture.0, limits(64, 10_000)).unwrap_err();
    assert!(source_is::<MetadataError>(&error));
    assert_display_contains(&error, "too large");
}

#[test]
fn extension_and_derived_path_rules_are_exact() {
    let fixture = Fixture::new();
    let selected = fixture.metadata("Assets/selected.cs", GUID_A);
    fs::write(fixture.path("Assets/ignored.META"), b"not metadata").unwrap();
    fs::write(fixture.path("Assets/ignoredmeta"), b"not metadata").unwrap();
    fs::create_dir(fixture.path("Assets/folder.meta")).unwrap();
    let index = valid_index(&fixture);
    assert_eq!(index.len(), 1);
    let selected_asset = selected.with_extension("");
    assert_eq!(
        index.asset_path(GUID_A.parse::<Guid>().unwrap()),
        Some(selected_asset.as_path())
    );
}

fn find_source<'a, T: Error + 'static>(error: &'a (dyn Error + 'static)) -> Option<&'a T> {
    let mut current = error.source();
    while let Some(source) = current {
        if let Some(found) = source.downcast_ref::<T>() {
            return Some(found);
        }
        current = source.source();
    }
    None
}
