use super::*;

impl EditorApp {
    /// Execute only a supported authoring edit addressed to its captured document scope.
    pub fn apply_inspector_action(
        &mut self,
        target: InspectorTarget,
        action: EditorAction,
    ) -> Result<(), String> {
        self.check_idle()?;
        self.validate_inspector_target(target)?;
        if !matches!(&action, EditorAction::Edit { id, .. } if *id == target.entity_id) {
            return Err("Inspector edit does not address its captured target.".into());
        }
        self.apply_action(action)?;
        self.inspector_panel = InspectorPanel::default();
        self.inspector_controller.draft(None, false);
        Ok(())
    }

    /// Execute an imported property/component edit with scope, liveness and owner validation.
    pub fn apply_imported_inspector_action(
        &mut self,
        target: InspectorTarget,
        action: imported::ImportedAction,
    ) -> Result<(), String> {
        self.check_idle()?;
        self.validate_inspector_target(target)?;
        let document = self
            .imported
            .as_ref()
            .ok_or("No imported document is active.")?;
        let property_edit = matches!(&action, imported::ImportedAction::EditEntity { id, .. } if *id == target.entity_id);
        let owned_script = match &action {
            imported::ImportedAction::SetScriptEnabled { script_file_id, .. } => document
                .components(target.entity_id)
                .map_err(|e| e.to_string())?
                .iter()
                .any(|c| {
                    matches!(c,
                    openunity_editor_core::imported_scene::ImportedComponent::Script(s)
                        if s.file_id == *script_file_id)
                }),
            _ => false,
        };
        if !property_edit && !owned_script {
            return Err(
                "Imported Inspector edit does not address its captured target/component.".into(),
            );
        }
        self.apply_imported_action(action)?;
        if property_edit {
            self.imported_inspector = imported::InspectorPanel::default();
            self.inspector_controller.draft(None, false);
        }
        Ok(())
    }
}
