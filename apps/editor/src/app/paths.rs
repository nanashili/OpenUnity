use super::*;

impl EditorApp {
    /// Validate an output path against current and archived source provenance.
    /// Missing archived source roots still exclude their future descendants.
    pub fn check_output_path(&self, path: &Path) -> Result<(), String> {
        self.policy.check_read(path).map_err(|e| e.to_string())?;
        let absolute = if path.is_absolute() {
            path.to_path_buf()
        } else {
            std::env::current_dir()
                .map_err(|e| e.to_string())?
                .join(path)
        };
        let normalized: PathBuf = absolute
            .components()
            .filter(|c| !matches!(c, std::path::Component::CurDir))
            .collect();
        if self
            .provenance_roots
            .iter()
            .chain(&self.protected_roots)
            .any(|root| normalized.starts_with(root))
        {
            return Err("Writes within a protected source or reference root are forbidden.".into());
        }
        Ok(())
    }

    pub(super) fn protect_candidate(&mut self, candidate: &Pending) {
        if let Pending::Imported(document, _) = candidate {
            let root = &document.source().project_root;
            if !self.provenance_roots.contains(root) {
                self.provenance_roots.push(root.clone());
            }
        }
    }

    pub(super) fn check_input(&self, path: &Path) -> Result<(), String> {
        self.policy.check_read(path).map_err(|e| e.to_string())?;
        let absolute =
            fs::canonicalize(path).map_err(|e| format!("open {}: {e}", path.display()))?;
        if self
            .forbidden_reads
            .iter()
            .any(|root| absolute.starts_with(root))
        {
            return Err(
                "Opening reference installation or restricted research inputs is unavailable."
                    .into(),
            );
        }
        Ok(())
    }
}
