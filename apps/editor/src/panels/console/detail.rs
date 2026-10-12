//! Reference detail semantics, clipped message content and retained scroll state.

use super::*;
use crate::console_detail::ConsoleDetailSnapshot;

impl ConsolePanel {
    pub(super) fn show_detail(
        &mut self,
        ui: &mut Ui,
        console: &Console,
        detail_rect: egui::Rect,
        clip: egui::Rect,
        lifetime: u64,
        valid_layout: bool,
    ) {
        // Validate again immediately before detail paint, including zero-size allocations.
        self.reconcile(console);
        let state = self.detail.snapshot().clone();
        self.show_detail_semantics(ui, detail_rect, lifetime, &state);
        let detail_clip = detail_rect.intersect(clip);
        if detail_clip.width() <= 0.0 || detail_clip.height() <= 0.0 {
            return;
        }
        let mut detail_ui = ui.new_child(
            egui::UiBuilder::new()
                .id_salt(("console.detail", lifetime))
                .max_rect(detail_rect),
        );
        detail_ui.set_clip_rect(detail_clip);
        let scroll = ScrollArea::vertical()
            .id_salt(("console_detail", lifetime))
            .auto_shrink([false, false])
            .min_scrolled_height(0.0)
            .max_height(detail_rect.height())
            .vertical_scroll_offset(state.detail_scroll_y)
            .show(&mut detail_ui, |ui| {
                if let (Some(message), Some(severity)) =
                    (&state.detail_message, state.detail_severity)
                {
                    ui.label(severity_label(severity));
                    ui.add(Label::new(message).wrap());
                } else {
                    ui.add(
                        Label::new(
                            state
                                .diagnostic
                                .as_deref()
                                .unwrap_or("Select a console record to view its full message."),
                        )
                        .wrap(),
                    );
                }
            });
        self.record_detail_scroll(&state, scroll, valid_layout);
    }

    fn show_detail_semantics(
        &mut self,
        ui: &mut Ui,
        detail_rect: egui::Rect,
        lifetime: u64,
        state: &ConsoleDetailSnapshot,
    ) {
        let semantic_text = match (&state.detail_message, state.detail_severity) {
            (Some(message), Some(severity)) => format!("[{}] {message}", severity_label(severity)),
            _ => "Select a console record to view its full message.".into(),
        };
        let semantic_response = ui.interact(
            detail_rect,
            ui.make_persistent_id(("console.detail.semantic", lifetime)),
            egui::Sense::hover(),
        );
        semantic_response.widget_info(|| {
            egui::WidgetInfo::labeled(egui::WidgetType::Label, ui.is_enabled(), &semantic_text)
        });
        self.record_control("console.detail", &semantic_response, false);
        if let Some(error) = &state.diagnostic {
            semantic_response.on_hover_text(error);
        }
    }

    fn record_detail_scroll(
        &mut self,
        state: &ConsoleDetailSnapshot,
        scroll: egui::scroll_area::ScrollAreaOutput<()>,
        valid_layout: bool,
    ) {
        let max_scroll_y = (scroll.content_size.y - scroll.inner_rect.height()).max(0.0);
        if valid_layout {
            let _ = self
                .detail
                .update_scroll(scroll.state.offset.y, max_scroll_y);
        }
        self.scroll_identity = state.selected;
        self.detail_scroll = Some(serde_json::json!({
            "inner_rect": crate::appearance::rect_json(scroll.inner_rect),
            "content_rect": crate::appearance::rect_json(egui::Rect::from_min_size(
                scroll.inner_rect.min - scroll.state.offset,
                scroll.content_size,
            )),
            "offset": [scroll.state.offset.x, scroll.state.offset.y],
            "max_scroll_y": max_scroll_y,
        }));
    }
}
