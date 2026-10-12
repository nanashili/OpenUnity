use crate::*;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

const GUID: &str = "0123456789abcdef0123456789abcdef";
const HEADER: &str = "fileFormatVersion: 2\nguid: 0123456789abcdef0123456789abcdef\n";

struct FixtureDir(PathBuf);

impl FixtureDir {
    fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let path = std::env::temp_dir().join(format!(
            "openunity-meta-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).unwrap();
        Self(path.canonicalize().unwrap())
    }
}

impl Drop for FixtureDir {
    fn drop(&mut self) {
        if let Err(error) = fs::remove_dir_all(&self.0) {
            if std::thread::panicking() {
                // Emergency reporting is best effort: preserve the original panic
                // even if stderr cannot be written during unwinding.
                let _ = writeln!(
                    std::io::stderr().lock(),
                    "metadata fixture cleanup failed during test failure: {error}"
                );
            } else {
                panic!("metadata fixture cleanup failed: {error}");
            }
        }
    }
}

#[test]
fn accepts_bom_crlf_and_ignores_importer_fields() {
    let fixture = concat!(
        "\u{feff}fileFormatVersion: 2\r\n",
        "guid: 0123456789ABCDEF0123456789ABCDEF\r\n",
        "DefaultImporter:\r\n",
        "  guid: deliberately not a GUID\r\n",
        "  fileFormatVersion: 17\r\n",
        "  uninterpreted importer data\r\n"
    );
    let metadata = parse(fixture.as_bytes()).unwrap();
    assert_eq!(metadata.guid.to_string(), GUID);
    assert_eq!(parse(HEADER.as_bytes()).unwrap(), metadata);
}

#[test]
fn rejects_invalid_guid_and_missing_or_duplicate_headers() {
    for invalid in [
        "",
        "0123",
        "0123456789abcdef0123456789abcdeg",
        "0123456789abcdef0123456789abcdef0",
        "0123456789abcdef0123456789abcdeé",
    ] {
        assert!(invalid.parse::<Guid>().is_err(), "{invalid}");
    }
    assert!(matches!(
        parse(b"fileFormatVersion: 2\n"),
        Err(MetadataError::MissingHeader("guid"))
    ));
    assert!(matches!(
        parse(format!("guid: {GUID}\n").as_bytes()),
        Err(MetadataError::MissingHeader("fileFormatVersion"))
    ));
    for (tail, key) in [
        ("fileFormatVersion: 2\n", "fileFormatVersion"),
        ("fileFormatVersion: 3\n", "fileFormatVersion"),
        ("guid: ffffffffffffffffffffffffffffffff\n", "guid"),
        (
            "DefaultImporter:\n  guid: ignored\nDefaultImporter:\n",
            "DefaultImporter",
        ),
        ("userData: first\nuserData: second\n", "userData"),
        ("userData: first\nuserData \t: second\n", "userData"),
    ] {
        assert!(
            matches!(parse(format!("{HEADER}{tail}").as_bytes()), Err(MetadataError::DuplicateHeader(found)) if found == key)
        );
    }
    assert!(matches!(
        parse(format!("fileFormatVersion: 3\nguid: {GUID}\n").as_bytes()),
        Err(MetadataError::UnsupportedVersion(_))
    ));
    assert!(matches!(
        parse(b"fileFormatVersion 2\n"),
        Err(MetadataError::MalformedHeader { line: 1 })
    ));
    assert!(matches!(
        parse(b"fileFormatVersion: 2\nguid: bad\n"),
        Err(MetadataError::InvalidGuid(_))
    ));
    assert!(matches!(
        parse(b"fileFormatVersion: 2\rguid: bad"),
        Err(MetadataError::MalformedHeader { .. })
    ));
}

#[test]
fn file_reader_reports_missing_invalid_utf8_and_size_limits() {
    let dir = FixtureDir::new();
    let path = dir.0.join("authored.meta");
    assert!(matches!(
        read(&path),
        Err(MetadataError::MissingMetadata(_))
    ));
    fs::write(&path, HEADER).unwrap();
    assert_eq!(read(&path).unwrap().guid.to_string(), GUID);
    fs::write(&path, [0xff, 0xfe]).unwrap();
    assert!(matches!(read(&path), Err(MetadataError::InvalidUtf8(_))));
    let mut boundary = HEADER.as_bytes().to_vec();
    boundary.push(b' ');
    boundary.resize(MAX_METADATA_BYTES, b'x');
    fs::write(&path, &boundary).unwrap();
    assert_eq!(read(&path).unwrap().guid.to_string(), GUID);
    boundary.push(b'x');
    fs::write(&path, &boundary).unwrap();
    assert!(matches!(read(&path), Err(MetadataError::TooLarge)));
    assert!(matches!(parse(&boundary), Err(MetadataError::TooLarge)));
    assert!(matches!(
        read(&dir.0),
        Err(MetadataError::NotRegularFile(_))
    ));
    assert!(matches!(
        read(dir.0.join("..")),
        Err(MetadataError::InvalidPath(_))
    ));
}

#[cfg(unix)]
#[test]
fn rejects_symlink_file_and_ancestor() {
    use std::os::unix::fs::symlink;
    let dir = FixtureDir::new();
    let real = dir.0.join("real");
    fs::create_dir(&real).unwrap();
    fs::write(real.join("authored.meta"), HEADER).unwrap();
    let linked_file = dir.0.join("linked.meta");
    symlink(real.join("authored.meta"), &linked_file).unwrap();
    assert!(matches!(read(&linked_file), Err(MetadataError::Symlink(path)) if path == linked_file));
    let linked_dir = dir.0.join("linked");
    symlink(&real, &linked_dir).unwrap();
    assert!(
        matches!(read(linked_dir.join("authored.meta")), Err(MetadataError::Symlink(path)) if path == linked_dir)
    );
}

#[test]
fn duplicate_guid_error_preserves_first_asset_and_both_paths() {
    let metadata = parse(HEADER.as_bytes()).unwrap();
    let mut index = MetadataIndex::new();
    index.insert("Textures/first.png", metadata).unwrap();
    index.insert("Textures/first.png", metadata).unwrap();
    let duplicate = index.insert("Textures/second.png", metadata).unwrap_err();
    assert_eq!(duplicate.guid.to_string(), GUID);
    assert_eq!(duplicate.first_path, Path::new("Textures/first.png"));
    assert_eq!(duplicate.duplicate_path, Path::new("Textures/second.png"));
    assert_eq!(index.len(), 1);
    assert_eq!(
        index.asset_path(metadata.guid),
        Some(Path::new("Textures/first.png"))
    );
}
