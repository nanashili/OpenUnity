use super::*;

impl EditorApp {
    /// Actual committed document-scoped Inspector view and last rendered controls.
    pub fn inspector_snapshot(&self) -> serde_json::Value {
        let mut value = self.inspector_controller.snapshot().to_json();
        value["profile"] = self.workspace.profile().name().into();
        value["controls"] = self.inspector_controls.clone().into();
        value["component_expansion"] = self.component_expansion.snapshot().to_json();
        value["component_menu_context"] = self
            .inspector_component_menu_context
            .map(|t| {
                serde_json::json!({
                    "document_scope": t.document_scope,
                    "entity_id": t.entity_id.0
                })
            })
            .into();
        value["draft"] = if self.imported.is_some() {
            self.imported_inspector.draft_snapshot()
        } else {
            self.inspector_panel.draft_snapshot()
        };
        value["panel_controls"] = if self.imported.is_some() {
            self.imported_inspector.controls()
        } else {
            self.inspector_panel.controls()
        }
        .to_vec()
        .into();
        value
    }

    /// Current explicitly addressed Inspector target.
    pub fn inspector_target(&self) -> Option<InspectorTarget> {
        self.inspector_controller.snapshot().target
    }

    /// Observe changed lock state after its target and draft state commit.
    pub fn observe_inspector_lock(&mut self, observer: impl FnMut(&InspectorSnapshot) + 'static) {
        self.inspector_controller.observe(observer);
    }

    /// Observe complete changed component presentation results after commit.
    pub fn observe_component_expansion(
        &mut self,
        observer: impl FnMut(&ComponentExpansionSnapshot) + 'static,
    ) {
        self.component_expansion.observe(observer);
    }

    pub(super) fn inspector_selection(&self) -> Option<InspectorTarget> {
        let id = self.imported.as_ref().map_or_else(
            || self.document.selection().primary(),
            ImportedDocument::primary,
        );
        id.map(|entity_id| InspectorTarget {
            document_scope: self.inspector_scope,
            entity_id,
        })
    }

    pub(super) fn reconcile_inspector(&mut self) {
        if self.workspace.profile() != AppearanceProfile::Reference {
            return;
        }
        let before = self.inspector_target();
        let selection = self.inspector_selection();
        let imported = &self.imported;
        let document = &self.document;
        self.inspector_controller
            .reconcile(self.inspector_scope, selection, |t| {
                imported.as_ref().map_or_else(
                    || document.scene().get(t.entity_id).is_some(),
                    |d| d.entity(t.entity_id).is_ok(),
                )
            });
        if before != self.inspector_target() {
            self.inspector_panel = InspectorPanel::default();
            self.imported_inspector = imported::InspectorPanel::default();
        }
        self.reconcile_component_expansion();
        self.inspector_controller.deliver_notifications();
    }

    /// Set Reference Inspector view state without mutating document or selection.
    pub fn set_inspector_lock(&mut self, locked: bool) -> Result<(), String> {
        self.check_idle()?;
        if self.workspace.profile() != AppearanceProfile::Reference {
            return Err("Inspector locking is unavailable in Legacy appearance.".into());
        }
        let before = self.inspector_target();
        let selection = self.inspector_selection();
        let imported = &self.imported;
        let document = &self.document;
        self.inspector_controller
            .set_lock(locked, self.inspector_scope, selection, |t| {
                imported.as_ref().map_or_else(
                    || document.scene().get(t.entity_id).is_some(),
                    |d| d.entity(t.entity_id).is_ok(),
                )
            })?;
        if before != self.inspector_target() {
            self.inspector_panel = InspectorPanel::default();
            self.imported_inspector = imported::InspectorPanel::default();
        }
        self.inspector_controller.deliver_notifications();
        self.reconcile_component_expansion();
        Ok(())
    }

    /// Toggle using current committed state, including activations from an open menu.
    pub fn toggle_inspector_lock(&mut self) -> Result<(), String> {
        self.set_inspector_lock(!self.inspector_controller.snapshot().locked)
    }

    pub(super) fn validate_inspector_target(&self, target: InspectorTarget) -> Result<(), String> {
        self.inspector_controller
            .validate_target(target, self.inspector_scope, |t| {
                self.imported.as_ref().map_or_else(
                    || self.document.scene().get(t.entity_id).is_some(),
                    |d| d.entity(t.entity_id).is_ok(),
                )
            })?;
        if self.inspector_target() != Some(target) {
            return Err("Inspector edit rejected: target is no longer displayed.".into());
        }
        if self
            .inspector_controller
            .snapshot()
            .draft_target
            .is_some_and(|owner| owner != target)
        {
            return Err("Inspector edit rejected: draft belongs to another target.".into());
        }
        Ok(())
    }
}
