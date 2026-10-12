use super::*;

mod dispatch;
mod inspector;
mod panes;

#[derive(Default)]
struct PanelActions {
    actions: Vec<EditorAction>,
    imported_actions: Vec<imported::ImportedAction>,
    ordered: Vec<Dispatch>,
}

enum Dispatch {
    Authoring(EditorAction),
    Imported(imported::ImportedAction),
    Inspector(InspectorTarget, EditorAction),
    ImportedInspector(InspectorTarget, imported::ImportedAction),
    ToggleInspectorLock,
    SetAllComponents(InspectorTarget, bool),
    SetComponent(InspectorTarget, i64, bool),
}

impl EditorApp {
    pub(super) fn panels(&mut self, ctx: &Context) {
        let mut pending = PanelActions::default();
        self.reconcile_inspector();
        self.inspector_controls.clear();
        let enabled = self.pending.is_none() && self.path_form.is_none();
        let reference = self.workspace.profile() == AppearanceProfile::Reference;
        self.viewport_panel
            .set_grid_visible(self.preferences.grid_visible);
        self.imported_viewport
            .set_grid_visible(self.preferences.grid_visible);
        let mut placeholder = Workspace::default();
        placeholder.set_profile(self.workspace.profile());
        let mut workspace = std::mem::replace(&mut self.workspace, placeholder);
        workspace.show_body(ctx, enabled, |ui, tab, header| {
            if header {
                pending
                    .ordered
                    .extend(self.show_inspector_header(ui).into_iter().map(
                        |request| match request {
                            InspectorHeaderAction::ToggleLock => Dispatch::ToggleInspectorLock,
                            InspectorHeaderAction::SetAllComponents { context, expanded } => {
                                Dispatch::SetAllComponents(context, expanded)
                            }
                        },
                    ));
                return;
            }
            self.render_tab(ui, tab, reference, &mut pending);
            if reference {
                pending
                    .ordered
                    .extend(pending.imported_actions.drain(..).map(Dispatch::Imported));
                pending
                    .ordered
                    .extend(pending.actions.drain(..).map(Dispatch::Authoring));
            }
        });
        self.workspace = workspace;
        self.preferences.grid_visible = if self.imported.is_some() {
            self.imported_viewport.grid_visible()
        } else {
            self.viewport_panel.grid_visible()
        };
        if enabled {
            self.dispatch_panel_actions(pending);
        }
    }

    fn render_tab(
        &mut self,
        ui: &mut egui::Ui,
        tab: TabId,
        reference: bool,
        pending: &mut PanelActions,
    ) {
        match tab {
            TabId::Hierarchy => self.hierarchy_pane(ui, reference, pending),
            TabId::Inspector => self.inspector_pane(ui, reference, pending),
            TabId::Scene => self.scene_pane(ui, pending),
            TabId::Game => self.game_pane(ui, reference),
            TabId::Project => self.project_panel.show(ui, self.project.as_ref()),
            TabId::Console => self.console_panel.show(ui, &mut self.console),
        }
    }
}
