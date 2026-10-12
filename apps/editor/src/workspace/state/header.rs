use super::*;
use crate::workspace::{DockId, DockSnapshot};
use eframe::egui::PointerButton;

impl Workspace {
    pub(super) fn draw_header(
        &mut self,
        ui: &mut Ui,
        dock: &DockSnapshot,
        tabs: &[TabId],
        enabled: bool,
    ) -> (Ui, TabId, Rect) {
        let id = dock.id;
        let header = dock.header;
        let mut active = dock.active;
        let inspector_actions =
            self.profile == AppearanceProfile::Reference && id == DockId::Inspector;
        let title_header = if inspector_actions {
            Rect::from_min_max(
                header.min,
                egui::pos2((header.right() - 48.0).max(header.left()), header.bottom()),
            )
        } else {
            header
        };
        let mut header_ui = self.header_ui(ui, id, title_header, enabled);
        active = self.draw_tabs(
            &mut header_ui,
            tabs,
            active,
            enabled,
            inspector_actions,
            title_header,
        );
        (header_ui, active, title_header)
    }

    fn header_ui(&self, ui: &mut Ui, id: DockId, title_header: Rect, enabled: bool) -> Ui {
        let mut header_ui = ui.new_child(
            egui::UiBuilder::new()
                .id_salt(("workspace.header", id as u8))
                .max_rect(title_header.shrink2(egui::vec2(
                    if self.profile == AppearanceProfile::Reference {
                        0.0
                    } else {
                        5.0
                    },
                    0.0,
                )))
                .layout(egui::Layout::left_to_right(egui::Align::Center)),
        );
        header_ui.set_clip_rect(title_header.intersect(ui.clip_rect()));
        if !enabled {
            header_ui.disable();
        }
        header_ui
    }

    fn draw_tabs(
        &mut self,
        header_ui: &mut Ui,
        tabs: &[TabId],
        mut active: TabId,
        enabled: bool,
        inspector_actions: bool,
        title_header: Rect,
    ) -> TabId {
        let first_control = self.snapshot.controls.len();
        if self.profile == AppearanceProfile::Reference {
            header_ui.spacing_mut().item_spacing.x = 0.0;
        }
        for tab in tabs {
            let response = if self.profile == AppearanceProfile::Reference {
                self.draw_reference_tab(header_ui, tab, active, inspector_actions, title_header)
            } else {
                header_ui.add_sized(
                    [72.0, 22.0],
                    egui::Button::new(tab.label()).selected(active == *tab),
                )
            };
            let pointer_down = response.is_pointer_button_down_on()
                && header_ui.input(|i| i.pointer.button_pressed(PointerButton::Primary));
            if enabled && (pointer_down || response.clicked()) {
                active = *tab;
            }
            self.record(tab.role(), &response, active == *tab);
        }
        for control in &mut self.snapshot.controls[first_control..] {
            control.selected = control.role == active.role();
        }
        for title in &mut self.snapshot.tab_titles {
            if tabs.contains(&title.tab_id) {
                title.selected = title.tab_id == active;
            }
        }
        active
    }

    pub(super) fn draw_header_actions(
        &self,
        ui: &mut Ui,
        id: DockId,
        header: Rect,
        title_header: Rect,
        enabled: bool,
        render: &mut impl FnMut(&mut Ui, TabId, bool),
    ) {
        let inspector_actions =
            self.profile == AppearanceProfile::Reference && id == DockId::Inspector;
        if inspector_actions {
            let action_rect =
                Rect::from_min_max(egui::pos2(title_header.right(), header.top()), header.max);
            let mut action_ui = ui.new_child(
                egui::UiBuilder::new()
                    .id_salt("workspace.inspector.actions")
                    .max_rect(action_rect)
                    .layout(egui::Layout::left_to_right(egui::Align::Center)),
            );
            action_ui.set_clip_rect(header.intersect(ui.clip_rect()));
            if !enabled {
                action_ui.disable();
            }
            render(&mut action_ui, TabId::Inspector, true);
        }
    }
}
