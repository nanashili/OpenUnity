use super::*;
use crate::workspace::{DockId, DockSnapshot};

impl Workspace {
    pub(super) fn draw(
        &mut self,
        ui: &mut Ui,
        body: Rect,
        enabled: bool,
        render: &mut impl FnMut(&mut Ui, TabId, bool),
    ) {
        let rects = self.prepare_layout(ui, body, enabled);
        for (id, rect, tabs) in [
            (DockId::Hierarchy, rects[0], &[TabId::Hierarchy][..]),
            (DockId::Scene, rects[1], &[TabId::Scene, TabId::Game][..]),
            (
                DockId::Project,
                rects[2],
                &[TabId::Project, TabId::Console][..],
            ),
            (DockId::Inspector, rects[3], &[TabId::Inspector][..]),
        ] {
            self.draw_dock(ui, id, rect, tabs, enabled, render);
        }
        self.snapshot.active_scene = self.scene;
        self.snapshot.active_bottom = self.bottom;
    }

    fn draw_dock(
        &mut self,
        ui: &mut Ui,
        id: DockId,
        rect: Rect,
        tabs: &[TabId],
        enabled: bool,
        render: &mut impl FnMut(&mut Ui, TabId, bool),
    ) {
        let header = Rect::from_min_size(rect.min, egui::vec2(rect.width(), 24.0));
        let content = Rect::from_min_max(egui::pos2(rect.left(), header.bottom()), rect.max);
        ui.painter().rect_filled(rect, 0.0, ui.visuals().panel_fill);
        ui.painter()
            .rect_filled(header, 0.0, ui.visuals().faint_bg_color);
        let active = match id {
            DockId::Scene => self.scene,
            DockId::Project => self.bottom,
            DockId::Hierarchy => TabId::Hierarchy,
            DockId::Inspector => TabId::Inspector,
        };
        let dock = DockSnapshot {
            id,
            rect,
            header,
            content,
            active,
        };
        let (_header_ui, active, title_header) = self.draw_header(ui, &dock, tabs, enabled);
        self.draw_header_actions(ui, id, header, title_header, enabled, render);
        match id {
            DockId::Scene => self.scene = active,
            DockId::Project => self.bottom = active,
            _ => (),
        }
        let _pane = self.draw_content(ui, id, content, active, enabled, render);
        self.snapshot.docks.push(DockSnapshot {
            id,
            rect,
            header,
            content,
            active,
        });
    }

    fn draw_content(
        &self,
        ui: &mut Ui,
        id: DockId,
        content: Rect,
        active: TabId,
        enabled: bool,
        render: &mut impl FnMut(&mut Ui, TabId, bool),
    ) -> Ui {
        let mut pane = ui.new_child(
            egui::UiBuilder::new()
                .id_salt(("workspace.content", id as u8))
                .max_rect(content.shrink2(egui::vec2(
                    if self.profile == AppearanceProfile::Reference {
                        4.0
                    } else {
                        6.0
                    },
                    if self.profile == AppearanceProfile::Reference {
                        0.0
                    } else {
                        6.0
                    },
                )))
                .layout(egui::Layout::top_down(egui::Align::Min)),
        );
        pane.set_clip_rect(if self.profile == AppearanceProfile::Reference {
            content.intersect(ui.clip_rect())
        } else {
            content.shrink(5.0).intersect(ui.clip_rect())
        });
        if !enabled {
            pane.disable();
        }
        render(&mut pane, active, false);
        pane
    }
}
