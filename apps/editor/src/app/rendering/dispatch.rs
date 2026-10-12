use super::*;

impl EditorApp {
    pub(super) fn dispatch_panel_actions(&mut self, pending: PanelActions) {
        for command in pending.ordered {
            let result = match command {
                Dispatch::Authoring(action) => self.apply_action(action),
                Dispatch::Imported(action) => self.apply_imported_action(action),
                Dispatch::Inspector(target, action) => self.apply_inspector_action(target, action),
                Dispatch::ImportedInspector(target, action) => {
                    self.apply_imported_inspector_action(target, action)
                }
                Dispatch::ToggleInspectorLock => self.toggle_inspector_lock(),
                Dispatch::SetAllComponents(context, expanded) => {
                    self.set_all_inspector_components_expanded(context, expanded)
                }
                Dispatch::SetComponent(context, file_id, expanded) => {
                    self.set_inspector_component_expanded(context, file_id, expanded)
                }
            };
            self.report(result, "Document panel action completed.");
        }
        for action in pending.imported_actions {
            let result = self.apply_imported_action(action);
            self.report(result, "Imported document action completed.");
        }
        for action in pending.actions {
            let result = self.apply_action(action);
            self.report(result, "Authoring action completed.");
        }
    }
}
