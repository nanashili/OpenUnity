use crate::{Guid, Metadata};
use std::collections::BTreeMap;
use std::error::Error;
use std::fmt;
use std::path::{Path, PathBuf};

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
