//! Retained source provenance and lexical write exclusion.

use std::path::{Component, Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::{ImportedError, Result};

/// Read-only diagnostic provenance and an enduring source write exclusion.
#[derive(Clone, Debug, Deserialize, Serialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ImportedSource {
    /// Absolute normalized original source root; availability is not required on reopen.
    pub project_root: PathBuf,
    /// Relative scene source path.
    pub scene: PathBuf,
    /// Relative explicit binding manifest path.
    pub bindings: PathBuf,
}
impl ImportedSource {
    pub(crate) fn validate(&self) -> Result<()> {
        validate_path(&self.project_root, true)?;
        validate_path(&self.scene, false)?;
        validate_path(&self.bindings, false)
    }
    /// Additive lexical exclusion, including when the source root is absent.
    /// The caller must also use WritePolicy for actual checked filesystem writes.
    pub fn forbids_write(&self, path: &Path) -> Result<bool> {
        self.validate()?;
        let absolute = if path.is_absolute() {
            path.to_owned()
        } else {
            std::env::current_dir()
                .map_err(|e| ImportedError::new("check source write", e))?
                .join(path)
        };
        validate_path(&absolute, true)?;
        Ok(absolute.starts_with(&self.project_root))
    }
}
fn validate_path(path: &Path, absolute: bool) -> Result<()> {
    let value = path
        .to_str()
        .ok_or_else(|| ImportedError::new("validate provenance", "path is not UTF-8"))?;
    if value.is_empty()
        || value.len() > engine_scene::MAX_STRING_BYTES
        || value.contains('\0')
        || path.is_absolute() != absolute
        || path
            .components()
            .any(|c| matches!(c, Component::ParentDir | Component::CurDir))
        || path.components().collect::<PathBuf>().to_str() != Some(value)
    {
        return Err(ImportedError::new(
            "validate provenance",
            "expected a normalized absolute root or nonempty relative source path",
        ));
    }
    Ok(())
}
