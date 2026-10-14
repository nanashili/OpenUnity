use super::*;

impl EditorApp {
    /// Apply the actual active document undo operation.
    pub fn undo(&mut self) -> Result<bool, String> {
        self.check_idle()?;
        let changed = if let Some(document) = &mut self.imported {
            document.undo().map_err(|e| e.to_string())?
        } else {
            self.document.undo().map_err(|e| e.to_string())?
        };
        if changed {
            if self.imported.is_none() {
                self.hierarchy_expansion.reconcile(self.document.scene());
                self.scene_visibility.reconcile(self.document.scene());
            }
            if self.workspace.profile() == AppearanceProfile::Legacy {
                self.imported_inspector = imported::InspectorPanel::default();
            }
            self.reconcile_inspector();
        }
        Ok(changed)
    }

    /// Apply the actual active document redo operation.
    pub fn redo(&mut self) -> Result<bool, String> {
        self.check_idle()?;
        let changed = if let Some(document) = &mut self.imported {
            document.redo().map_err(|e| e.to_string())?
        } else {
            self.document.redo().map_err(|e| e.to_string())?
        };
        if changed {
            if self.imported.is_none() {
                self.hierarchy_expansion.reconcile(self.document.scene());
                self.scene_visibility.reconcile(self.document.scene());
            }
            if self.workspace.profile() == AppearanceProfile::Legacy {
                self.imported_inspector = imported::InspectorPanel::default();
            }
            self.reconcile_inspector();
        }
        Ok(changed)
    }

    pub(super) fn can_undo(&self) -> bool {
        self.imported
            .as_ref()
            .map_or_else(|| self.document.can_undo(), ImportedDocument::can_undo)
    }

    pub(super) fn can_redo(&self) -> bool {
        self.imported
            .as_ref()
            .map_or_else(|| self.document.can_redo(), ImportedDocument::can_redo)
    }
}
