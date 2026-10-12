use super::*;

impl EditorApp {
    /// Path of the successfully opened or saved document.
    pub fn save_path(&self) -> Option<&Path> {
        self.save_path.as_deref()
    }

    /// Persist to an explicit path. Save As never replaces an existing file.
    pub fn save_to(&mut self, path: &Path, save_as: bool) -> Result<(), String> {
        if save_as || self.save_path.as_deref() != Some(path) {
            match fs::symlink_metadata(path) {
                Ok(_) => {
                    return Err("Save As requires a new path; existing files are preserved.".into());
                }
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(error) => return Err(format!("save {}: {error}", path.display())),
            }
        }
        self.check_output_path(path)?;
        if let Some(document) = &mut self.imported {
            imported_scene_io::save(path, document.snapshot(), &self.policy)
                .map_err(|e| e.to_string())?;
            document.mark_saved();
        } else {
            scene_io::save(path, self.document.scene(), &self.policy).map_err(|e| e.to_string())?;
            self.document.mark_saved();
        }
        self.save_path = Some(path.to_path_buf());
        self.scene_text = path.display().to_string();
        Ok(())
    }

    pub(super) fn save_current(&mut self) -> Result<(), String> {
        if let Some(path) = self.save_path.clone() {
            self.save_to(&path, false)
        } else if !self.scene_text.trim().is_empty() {
            self.save_to(&PathBuf::from(self.scene_text.trim()), true)
        } else {
            Err("Enter a new independent .ouscene or .ouimport path before saving.".into())
        }
    }
}
