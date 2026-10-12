//! Bounded, read-only metadata headers; this is not a YAML parser or asset importer.

use std::collections::BTreeMap;
use std::error::Error;
use std::fmt;
use std::fs::{self, File};
use std::io::{self, Read};
use std::path::{Component, Path, PathBuf};
use std::str::FromStr;

/// Maximum metadata input and maximum bytes read from one metadata file.
pub const MAX_METADATA_BYTES: usize = 1024 * 1024;

/// A validated 32-hexadecimal-digit GUID, displayed in lowercase.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Hash)]
pub struct Guid([u8; 16]);

/// A GUID was not exactly 32 ASCII hexadecimal digits.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct InvalidGuid;

impl fmt::Display for InvalidGuid {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("GUID must contain exactly 32 hexadecimal digits")
    }
}

impl Error for InvalidGuid {}

impl FromStr for Guid {
    type Err = InvalidGuid;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        if value.len() != 32 || !value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            return Err(InvalidGuid);
        }
        let mut bytes = [0; 16];
        for (output, pair) in bytes.iter_mut().zip(value.as_bytes().chunks_exact(2)) {
            let digit = |byte: u8| match byte {
                b'0'..=b'9' => byte - b'0',
                b'a'..=b'f' => byte - b'a' + 10,
                b'A'..=b'F' => byte - b'A' + 10,
                _ => unreachable!("validated hexadecimal digit"),
            };
            *output = (digit(pair[0]) << 4) | digit(pair[1]);
        }
        Ok(Self(bytes))
    }
}

impl fmt::Display for Guid {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for byte in self.0 {
            write!(f, "{byte:02x}")?;
        }
        Ok(())
    }
}

/// The supported, validated part of a metadata file.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Metadata {
    /// Asset identity supplied by the metadata header.
    pub guid: Guid,
}

/// A missing, unsupported, malformed, or inaccessible metadata header.
#[derive(Debug)]
pub enum MetadataError {
    /// The metadata file or one of its required ancestors does not exist.
    MissingMetadata(PathBuf),
    /// Filesystem access failed.
    Io {
        /// Path involved in the failed operation.
        path: PathBuf,
        /// Original filesystem error.
        source: io::Error,
    },
    /// A file or ancestor is a symbolic link.
    Symlink(PathBuf),
    /// The supplied path is empty or contains parent traversal.
    InvalidPath(PathBuf),
    /// The supplied metadata path is not a regular file.
    NotRegularFile(PathBuf),
    /// Input exceeds [`MAX_METADATA_BYTES`].
    TooLarge,
    /// Input is not UTF-8.
    InvalidUtf8(std::str::Utf8Error),
    /// A required top-level header was absent.
    MissingHeader(&'static str),
    /// A supported top-level header appeared more than once.
    DuplicateHeader(&'static str),
    /// The declared version is not the supported literal `2`.
    UnsupportedVersion(String),
    /// A top-level line has no key/value separator or uses unsupported line endings.
    MalformedHeader {
        /// One-based input line number.
        line: usize,
    },
    /// The GUID value is malformed.
    InvalidGuid(InvalidGuid),
}

impl fmt::Display for MetadataError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingMetadata(path) => write!(f, "missing metadata: {}", path.display()),
            Self::Io { path, source } => write!(f, "cannot read {}: {source}", path.display()),
            Self::Symlink(path) => write!(f, "metadata path is a symlink: {}", path.display()),
            Self::InvalidPath(path) => write!(f, "invalid metadata path: {}", path.display()),
            Self::NotRegularFile(path) => {
                write!(f, "metadata is not a regular file: {}", path.display())
            }
            Self::TooLarge => write!(f, "metadata exceeds {MAX_METADATA_BYTES} bytes"),
            Self::InvalidUtf8(source) => write!(f, "metadata is not UTF-8: {source}"),
            Self::MissingHeader(key) => write!(f, "metadata is missing {key}"),
            Self::DuplicateHeader(key) => write!(f, "duplicate metadata header: {key}"),
            Self::UnsupportedVersion(version) => {
                write!(f, "unsupported metadata fileFormatVersion: {version}")
            }
            Self::MalformedHeader { line } => write!(f, "malformed metadata header at line {line}"),
            Self::InvalidGuid(source) => source.fmt(f),
        }
    }
}

impl Error for MetadataError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Io { source, .. } => Some(source),
            Self::InvalidUtf8(source) => Some(source),
            Self::InvalidGuid(source) => Some(source),
            _ => None,
        }
    }
}

/// Reads the supported UTF-8 header dialect from caller-supplied bytes.
///
/// An optional initial BOM and LF/CRLF line endings are accepted. Exactly one
/// top-level `fileFormatVersion: 2` and `guid` are required. Indented importer
/// lines and other top-level key/value fields are ignored, without interpreting
/// their contents. No complete YAML or Unity serialization support is implied.
///
/// # Errors
/// Returns a structured error for oversized, non-UTF-8, missing, duplicated,
/// malformed, or unsupported headers.
pub fn parse(bytes: &[u8]) -> Result<Metadata, MetadataError> {
    if bytes.len() > MAX_METADATA_BYTES {
        return Err(MetadataError::TooLarge);
    }
    let text = std::str::from_utf8(bytes).map_err(MetadataError::InvalidUtf8)?;
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    let mut has_version = false;
    let mut guid = None;
    for (index, line) in text.lines().enumerate() {
        if line.contains('\r') {
            return Err(MetadataError::MalformedHeader { line: index + 1 });
        }
        if line.starts_with([' ', '\t']) || line.is_empty() || line.starts_with('#') {
            continue;
        }
        let (key, value) = line
            .split_once(':')
            .ok_or(MetadataError::MalformedHeader { line: index + 1 })?;
        let key = key.trim_end_matches([' ', '\t']);
        let value = value.trim_matches([' ', '\t']);
        match key {
            "fileFormatVersion" => {
                if has_version {
                    return Err(MetadataError::DuplicateHeader("fileFormatVersion"));
                }
                has_version = true;
                if value != "2" {
                    return Err(MetadataError::UnsupportedVersion(value.to_owned()));
                }
            }
            "guid" => {
                if guid.is_some() {
                    return Err(MetadataError::DuplicateHeader("guid"));
                }
                guid = Some(value.parse().map_err(MetadataError::InvalidGuid)?);
            }
            "" => return Err(MetadataError::MalformedHeader { line: index + 1 }),
            _ => {}
        }
    }
    if !has_version {
        return Err(MetadataError::MissingHeader("fileFormatVersion"));
    }
    Ok(Metadata {
        guid: guid.ok_or(MetadataError::MissingHeader("guid"))?,
    })
}

/// Reads an explicit metadata file without importing or modifying assets.
///
/// All supplied path components, including ancestors, must be nonsymlinks. Empty
/// paths and parent traversal are rejected. At most [`MAX_METADATA_BYTES`] are
/// read. Concurrent hostile filesystem mutation is outside this reader's
/// guarantee; filesystem checks do not provide an atomic sandbox boundary.
///
/// # Errors
/// Reports missing metadata, symlinks, nonfiles, access failures, or any error
/// from [`parse`].
pub fn read(path: impl AsRef<Path>) -> Result<Metadata, MetadataError> {
    let path = path.as_ref();
    if path.as_os_str().is_empty()
        || path
            .components()
            .any(|component| matches!(component, Component::ParentDir))
    {
        return Err(MetadataError::InvalidPath(path.to_owned()));
    }
    let absolute = if path.is_absolute() {
        path.to_owned()
    } else {
        std::env::current_dir()
            .map_err(|source| io_error(path, source))?
            .join(path)
    };
    let mut checked = PathBuf::new();
    let mut components = absolute.components().peekable();
    while let Some(component) = components.next() {
        checked.push(component.as_os_str());
        let metadata =
            fs::symlink_metadata(&checked).map_err(|source| io_error(&checked, source))?;
        if metadata.file_type().is_symlink() {
            return Err(MetadataError::Symlink(checked));
        }
        // Reject special files before opening: opening a FIFO could block.
        if components.peek().is_none() && !metadata.is_file() {
            return Err(MetadataError::NotRegularFile(path.to_owned()));
        }
    }
    let file = File::open(path).map_err(|source| io_error(path, source))?;
    let metadata = file.metadata().map_err(|source| io_error(path, source))?;
    if !metadata.is_file() {
        return Err(MetadataError::NotRegularFile(path.to_owned()));
    }
    if metadata.len() > MAX_METADATA_BYTES as u64 {
        return Err(MetadataError::TooLarge);
    }
    let mut bytes = Vec::new();
    (&file)
        .take(MAX_METADATA_BYTES as u64)
        .read_to_end(&mut bytes)
        .map_err(|source| io_error(path, source))?;
    if file
        .metadata()
        .map_err(|source| io_error(path, source))?
        .len()
        > MAX_METADATA_BYTES as u64
    {
        return Err(MetadataError::TooLarge);
    }
    parse(&bytes)
}

fn io_error(path: &Path, source: io::Error) -> MetadataError {
    if source.kind() == io::ErrorKind::NotFound {
        MetadataError::MissingMetadata(path.to_owned())
    } else {
        MetadataError::Io {
            path: path.to_owned(),
            source,
        }
    }
}

/// Two distinct caller-supplied asset paths claim the same GUID.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DuplicateGuid {
    /// Conflicting GUID.
    pub guid: Guid,
    /// The asset path already recorded by the index.
    pub first_path: PathBuf,
    /// The distinct asset path whose insertion failed.
    pub duplicate_path: PathBuf,
}

impl fmt::Display for DuplicateGuid {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "duplicate GUID {} for {} and {}",
            self.guid,
            self.first_path.display(),
            self.duplicate_path.display()
        )
    }
}

impl Error for DuplicateGuid {}

/// A caller-populated snapshot of GUID ownership; performs no asset discovery.
/// Paths are compared as supplied and are not canonicalized or opened.
#[derive(Clone, Debug, Default)]
pub struct MetadataIndex {
    assets: BTreeMap<Guid, PathBuf>,
}

impl MetadataIndex {
    /// Creates an empty GUID ownership index.
    pub fn new() -> Self {
        Self::default()
    }

    /// Records metadata for an asset path. Repeating the same pair is a no-op.
    ///
    /// # Errors
    /// A distinct path with the same GUID returns both paths without replacing
    /// the original index entry. Build a new snapshot when refreshing metadata.
    pub fn insert(
        &mut self,
        asset_path: impl AsRef<Path>,
        metadata: Metadata,
    ) -> Result<(), DuplicateGuid> {
        let asset_path = asset_path.as_ref();
        if let Some(first_path) = self.assets.get(&metadata.guid) {
            return if first_path == asset_path {
                Ok(())
            } else {
                Err(DuplicateGuid {
                    guid: metadata.guid,
                    first_path: first_path.clone(),
                    duplicate_path: asset_path.to_owned(),
                })
            };
        }
        self.assets.insert(metadata.guid, asset_path.to_owned());
        Ok(())
    }

    /// Returns the original asset path associated with a GUID.
    pub fn asset_path(&self, guid: Guid) -> Option<&Path> {
        self.assets.get(&guid).map(PathBuf::as_path)
    }

    /// Number of distinct recorded GUIDs.
    pub fn len(&self) -> usize {
        self.assets.len()
    }

    /// Whether no GUIDs have been recorded.
    pub fn is_empty(&self) -> bool {
        self.assets.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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
            let _ = fs::remove_dir_all(&self.0);
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
        assert!(
            matches!(read(&linked_file), Err(MetadataError::Symlink(path)) if path == linked_file)
        );
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
}
