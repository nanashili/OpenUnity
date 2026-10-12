use super::*;
use crate::appearance;
use crate::workspace::TabTitleSnapshot;

impl Workspace {
    pub(super) fn draw_reference_tab(
        &mut self,
        header_ui: &mut Ui,
        tab: &TabId,
        active: TabId,
        inspector_actions: bool,
        title_header: Rect,
    ) -> Response {
        let selected = active == *tab;
        let font = egui::FontId::proportional(12.0);
        let text = header_ui.painter().layout_no_wrap(
            tab.label().to_owned(),
            font.clone(),
            appearance::TEXT,
        );
        let width = (text.size().x + 16.0 + 4.0 + 16.0).min(if inspector_actions {
            title_header.width()
        } else {
            f32::INFINITY
        });
        let response = header_ui.add_sized(
            [width, 24.0],
            egui::Button::new("")
                .selected(selected)
                .fill(if selected {
                    appearance::BAR
                } else {
                    appearance::CHROME
                })
                .stroke(egui::Stroke::NONE)
                .corner_radius(0.0),
        );
        let icon_rect = egui::Rect::from_min_size(
            egui::pos2(response.rect.left() + 8.0, response.rect.center().y - 8.0),
            egui::vec2(16.0, 16.0),
        );
        let icon_role = match tab {
            TabId::Hierarchy => "tab_hierarchy",
            TabId::Scene => "tab_scene",
            TabId::Game => "tab_game",
            TabId::Project => "tab_project",
            TabId::Console => "tab_console",
            TabId::Inspector => "tab_inspector",
        };
        crate::resources::paint(header_ui, icon_role, tab.role(), response.id, icon_rect);
        self.paint_tab_title(header_ui, tab, response, icon_rect, selected, font)
    }

    fn paint_tab_title(
        &mut self,
        header_ui: &mut Ui,
        tab: &TabId,
        response: Response,
        icon_rect: Rect,
        selected: bool,
        font: egui::FontId,
    ) -> Response {
        let text_left = icon_rect.right() + 4.0;
        let title_rect = Rect::from_min_max(
            egui::pos2(text_left, response.rect.top()),
            egui::pos2(
                (response.rect.right() - 8.0).max(text_left),
                response.rect.bottom(),
            ),
        );
        let presenter = &mut self.title_presenters[tab.index()];
        let presentation = presenter
            .update(
                header_ui.ctx(),
                &self.titles[tab.index()],
                title_rect.width(),
                font,
            )
            .clone();
        response.widget_info(|| {
            egui::WidgetInfo::selected(
                egui::WidgetType::SelectableLabel,
                header_ui.is_enabled(),
                selected,
                &presentation.accessible_name,
            )
        });
        let painted_text_rect = presenter.paint(header_ui.painter(), title_rect, appearance::TEXT);
        let response = if let Some(tooltip) = &presentation.tooltip {
            response.on_hover_text(tooltip)
        } else {
            response
        };
        self.snapshot.tab_titles.push(TabTitleSnapshot {
            tab_id: *tab,
            presentation,
            title_rect,
            icon_rect,
            widget_rect: response.rect,
            widget_id: response.id,
            selected,
            painted_text_rect,
        });
        if selected {
            header_ui.painter().line_segment(
                [response.rect.left_top(), response.rect.right_top()],
                egui::Stroke::new(1.0, appearance::ACCENT),
            );
        }
        response
    }
}
