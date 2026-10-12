use super::*;

impl EditorApp {
    pub(super) fn reconcile_component_expansion(&mut self) {
        let mut live = Vec::new();
        if let Some(document) = &self.imported {
            let supported: std::collections::BTreeSet<_> = document
                .plan()
                .transforms
                .iter()
                .map(|c| c.file_id)
                .chain(document.plan().scripts.iter().map(|c| c.file_id))
                .collect();
            let objects: std::collections::BTreeMap<_, _> = document
                .plan()
                .game_objects
                .iter()
                .map(|g| (g.file_id, g))
                .collect();
            for mapping in document.snapshot().editor_ids() {
                if let Some(object) = objects.get(&mapping.game_object_file_id) {
                    live.push((
                        InspectorTarget {
                            document_scope: self.inspector_scope,
                            entity_id: mapping.entity_id,
                        },
                        object
                            .components
                            .iter()
                            .filter(|id| supported.contains(id))
                            .map(|id| (*id, true))
                            .collect(),
                    ));
                }
            }
        }
        self.component_expansion
            .reconcile(self.inspector_scope, self.inspector_target(), &live);
    }

    /// Apply an explicitly scoped individual component presentation request.
    pub fn set_inspector_component_expanded(
        &mut self,
        context: InspectorTarget,
        file_id: i64,
        expanded: bool,
    ) -> Result<(), String> {
        self.check_idle()?;
        if self.workspace.profile() != AppearanceProfile::Reference {
            return Err("Component expansion is unavailable in Legacy appearance.".into());
        }
        self.reconcile_inspector();
        self.component_expansion
            .set_component(context, file_id, expanded)
    }

    /// Apply a bulk request to current live membership of a captured displayed target.
    pub fn set_all_inspector_components_expanded(
        &mut self,
        context: InspectorTarget,
        expanded: bool,
    ) -> Result<(), String> {
        self.check_idle()?;
        if self.workspace.profile() != AppearanceProfile::Reference {
            return Err("Component expansion is unavailable in Legacy appearance.".into());
        }
        self.reconcile_inspector();
        self.component_expansion.set_all(context, expanded)
    }
}
