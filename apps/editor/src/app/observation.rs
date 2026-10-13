use super::*;

impl EditorApp {
    /// Legacy authoring-only view. Use imported_document for the active imported mode.
    /// This inactive legacy snapshot is never a save or mutation source in imported mode.
    pub fn document(&self) -> &Document {
        &self.document
    }

    /// Active typed imported document, or None in independent primitive authoring mode.
    pub fn imported_document(&self) -> Option<&ImportedDocument> {
        self.imported.as_ref()
    }

    /// Actual rendered workspace geometry and accessible control identities.
    pub fn workspace_snapshot(&self) -> &WorkspaceSnapshot {
        self.workspace.snapshot()
    }

    /// Replace only one tab's independent session title metadata, atomically.
    pub fn replace_tab_title(
        &mut self,
        tab: TabId,
        metadata: crate::tab_titles::TabTitleMetadata,
    ) -> Result<(), String> {
        self.workspace.replace_tab_title(tab, metadata)
    }

    /// Restore one tab's fixed clean title, without changing document or Inspector state.
    pub fn clear_tab_title(&mut self, tab: TabId) {
        self.workspace.clear_tab_title(tab);
    }

    /// Read one complete title input record retained for this UI session.
    pub fn tab_title_metadata(&self, tab: TabId) -> &crate::tab_titles::TabTitleMetadata {
        self.workspace.tab_title_metadata(tab)
    }

    /// Select a reviewed shell profile without replacing documents or Inspector drafts.
    pub fn set_appearance_profile(&mut self, profile: AppearanceProfile) {
        self.workspace.set_profile(profile);
        self.reconcile_inspector();
    }

    /// Actual last-frame Project browser state, paths, and rendered controls.
    pub fn project_browser_snapshot(&self) -> serde_json::Value {
        self.project_panel.snapshot(self.project.as_ref())
    }

    /// Actual last-frame hierarchy query, visible canonical identities, and controls.
    pub fn hierarchy_browser_snapshot(&self) -> serde_json::Value {
        self.hierarchy_browser.snapshot()
    }

    /// Actual shell appearance and Inspector scroll allocations from the last frame.
    pub fn appearance_snapshot(&self) -> serde_json::Value {
        serde_json::json!({
            "profile": self.workspace.profile().name(),
            "inspector_scroll": self.inspector_scroll,
            "resources": self.resource_snapshot
        })
    }

    /// Whether the active document differs from its last successful save.
    pub fn is_dirty(&self) -> bool {
        self.imported
            .as_ref()
            .map_or_else(|| self.document.is_dirty(), ImportedDocument::is_dirty)
    }

    /// Full active semantic archive, without transient selection or history.
    pub fn semantic_snapshot(&self) -> Result<serde_json::Value, String> {
        let bytes = if let Some(document) = &self.imported {
            imported_scene_io::encode(document.snapshot()).map_err(|e| e.to_string())?
        } else {
            scene_io::encode(self.document.scene()).map_err(|e| e.to_string())?
        };
        serde_json::from_slice(&bytes).map_err(|e| e.to_string())
    }
}
