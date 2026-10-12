use std::{collections::BTreeSet, fmt};

/// Maximum scene, binding manifest, or transfer payload bytes.
pub const MAX_INPUT_BYTES: usize = 16 * 1024 * 1024;
/// Maximum documents, graph objects, or discovered project entries.
pub const MAX_OBJECTS: usize = 10_000;
/// Maximum parser or hierarchy depth.
pub const MAX_DEPTH: usize = 64;
/// Maximum UTF-8 bytes in one string.
pub const MAX_STRING_BYTES: usize = 1024 * 1024;
/// Maximum serialized fields on one script.
pub const MAX_FIELDS: usize = 256;
/// Explicit malformed, unsupported, or inaccessible scene input.
#[derive(Debug)]
pub struct SceneError(String);
impl SceneError {
    /// Construct an explicit input diagnostic at a serialization or importer boundary.
    pub fn new(detail: impl ToString) -> Self {
        Self(detail.to_string())
    }
}
impl fmt::Display for SceneError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}
impl std::error::Error for SceneError {}
pub(crate) type Result<T> = std::result::Result<T, SceneError>;
/// Validate the shared UTF-8 byte bound and embedded-NUL restriction.
pub fn validate_text(value: &str) -> Result<()> {
    if value.len() > MAX_STRING_BYTES || value.contains('\0') {
        return Err(SceneError::new("oversized string or embedded NUL"));
    }
    Ok(())
}
pub(crate) fn unique_ids(ids: impl IntoIterator<Item = i64>) -> Result<BTreeSet<i64>> {
    let mut set = BTreeSet::new();
    for id in ids {
        if id <= 0 || !set.insert(id) {
            return Err(SceneError::new("invalid or duplicate fileID"));
        }
    }
    Ok(set)
}
