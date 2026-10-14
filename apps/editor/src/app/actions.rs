use super::*;

impl EditorApp {
    /// Execute an imported inspector/selection transaction on the authoritative plan.
    pub fn apply_imported_action(
        &mut self,
        action: imported::ImportedAction,
    ) -> Result<(), String> {
        self.check_idle()?;
        if matches!(
            action,
            imported::ImportedAction::SetComponentExpanded { .. }
        ) {
            return Err("Component expansion requires a scoped Inspector request.".into());
        }
        let document = self
            .imported
            .as_mut()
            .ok_or("No imported document is active.")?;
        match action {
            imported::ImportedAction::SetComponentExpanded { .. } => {
                unreachable!("checked scoped presentation request")
            }
            imported::ImportedAction::Select { id, toggle } => document.select(id, toggle),
            imported::ImportedAction::EditEntity { id, edit } => {
                document.edit_entity(id, edit).map(|_| ())
            }
            imported::ImportedAction::SetScriptEnabled {
                script_file_id,
                enabled,
            } => document
                .set_script_enabled(script_file_id, enabled)
                .map(|_| ()),
        }
        .map_err(|e| e.to_string())?;
        self.reconcile_inspector();
        Ok(())
    }

    /// Execute a panel transaction against the one live document and history.
    pub fn apply_action(&mut self, action: EditorAction) -> Result<(), String> {
        self.check_idle()?;
        if self.imported.is_some() {
            return match action {
                EditorAction::Select { id, toggle } => {
                    self.apply_imported_action(imported::ImportedAction::Select {
                        id: Some(id),
                        toggle,
                    })
                }
                _ => Err("Structural, primitive, Euler and generic authoring edits are unavailable for imported documents.".into()),
            };
        }
        self.apply_authoring_action(action)?;
        self.reconcile_inspector();
        Ok(())
    }

    fn apply_authoring_action(&mut self, action: EditorAction) -> Result<(), String> {
        match action {
            EditorAction::Select { id, toggle } => {
                let (scene, selection) = self.document.selection_mut();
                if toggle {
                    selection.toggle(id, scene)
                } else {
                    selection.replace(Some(id), scene)
                }
                .map_err(|e| e.to_string())?;
            }
            EditorAction::Create { primitive, parent } => {
                self.create_authoring_entity(primitive, parent)?;
            }
            EditorAction::DeleteSelection => {
                self.delete_authoring_selection()?;
            }
            EditorAction::CreateParentForSelection => {
                self.create_parent_for_selection()?;
            }
            EditorAction::Edit {
                id,
                name,
                active,
                transform,
            } => {
                self.document
                    .edit("Edit entity", |scene| {
                        scene.edit(id, |entity| {
                            entity.name = name;
                            entity.active = active;
                            entity.transform = transform;
                        })
                    })
                    .map_err(|e| e.to_string())?;
            }
            EditorAction::Reparent { id, parent } => {
                self.document
                    .edit("Reparent entity", |scene| {
                        scene.edit(id, |entity| entity.parent = parent)
                    })
                    .map_err(|e| e.to_string())?;
            }
            EditorAction::SetSiblingIndex { id, sibling_index } => {
                self.document
                    .edit("Reorder sibling", |scene| {
                        scene.set_sibling_index(id, sibling_index).map(|_| ())
                    })
                    .map_err(|e| e.to_string())?;
            }
        }
        Ok(())
    }

    fn create_authoring_entity(
        &mut self,
        primitive: openunity_editor_core::scene::Primitive,
        parent: Option<openunity_editor_core::scene::EntityId>,
    ) -> Result<(), String> {
        let mut created = None;
        self.document
            .edit("Create entity", |scene| {
                created = Some(scene.create(format!("{primitive:?}"), primitive, parent)?);
                Ok(())
            })
            .map_err(|e| e.to_string())?;
        let (scene, selection) = self.document.selection_mut();
        selection
            .replace(created, scene)
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    fn delete_authoring_selection(&mut self) -> Result<(), String> {
        let ids: Vec<_> = self.document.selection().ids().collect();
        self.document
            .edit("Delete selection", |scene| {
                for id in ids {
                    if scene.get(id).is_some() {
                        scene.remove_subtree(id)?;
                    }
                }
                Ok(())
            })
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    fn create_parent_for_selection(&mut self) -> Result<(), String> {
        let selected_ids: Vec<_> = self.document.selection().ids().collect();
        let mut new_parent = None;

        self.document
            .edit("Create parent for selection", |scene| {
                new_parent = Some(scene.create_parent_for_siblings(&selected_ids, "New Parent")?);
                Ok(())
            })
            .map_err(|error| error.to_string())?;

        let new_parent = new_parent.ok_or("Parent creation completed without an entity.")?;
        let (scene, selection) = self.document.selection_mut();
        selection
            .replace(Some(new_parent), scene)
            .map_err(|error| error.to_string())?;

        Ok(())
    }
}
