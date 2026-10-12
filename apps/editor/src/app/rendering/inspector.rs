use super::*;

impl EditorApp {
    pub(super) fn inspector_pane(
        &mut self,
        ui: &mut egui::Ui,
        reference: bool,
        pending: &mut PanelActions,
    ) {
        let scroll = egui::ScrollArea::vertical().id_salt("workspace.inspector.scroll");
        let scroll = scroll.show(ui, |ui| {
            if reference {
                self.reference_inspector(ui, pending);
            } else if let Some(document) = &self.imported {
                pending
                    .imported_actions
                    .extend(self.imported_inspector.show(ui, document));
            } else {
                pending.actions.extend(self.inspector_panel.show(
                    ui,
                    self.document.scene(),
                    self.document.selection(),
                ));
            }
        });
        self.inspector_scroll = serde_json::json!({
            "inner_rect": appearance::rect_json(scroll.inner_rect),
            "content_rect": appearance::rect_json(egui::Rect::from_min_size(
                scroll.inner_rect.min - scroll.state.offset,
                scroll.content_size
            )),
            "offset": [scroll.state.offset.x, scroll.state.offset.y]
        });
    }

    fn reference_inspector(&mut self, ui: &mut egui::Ui, pending: &mut PanelActions) {
        if let Some(notice) = &self.inspector_controller.snapshot().notice {
            ui.weak(notice);
        }
        let target = self.inspector_target();
        if let Some(document) = &self.imported {
            let emitted = self.imported_inspector.show_target(
                ui,
                document,
                target.map(|t| t.entity_id),
                self.component_expansion.snapshot(),
            );
            for action in emitted {
                if let imported::ImportedAction::SetComponentExpanded { file_id, expanded } = action
                {
                    if let Some(target) = target {
                        pending
                            .ordered
                            .push(Dispatch::SetComponent(target, file_id, expanded));
                    }
                } else if matches!(action, imported::ImportedAction::Select { .. }) {
                    pending.ordered.push(Dispatch::Imported(action));
                } else if let Some(target) = target {
                    pending
                        .ordered
                        .push(Dispatch::ImportedInspector(target, action));
                }
            }
        } else {
            let emitted = self.inspector_panel.show_target(
                ui,
                self.document.scene(),
                self.document.selection(),
                target.map(|t| t.entity_id),
            );
            if let Some(target) = target {
                pending.ordered.extend(
                    emitted
                        .into_iter()
                        .map(|action| Dispatch::Inspector(target, action)),
                );
            }
        }
        let (owner, changed) = if self.imported.is_some() {
            self.imported_inspector.draft_state()
        } else {
            self.inspector_panel.draft_state()
        };
        self.inspector_controller.draft(
            owner.map(|entity_id| InspectorTarget {
                document_scope: self.inspector_scope,
                entity_id,
            }),
            changed,
        );
    }
}
