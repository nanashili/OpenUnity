//! Reference pane subdivision and retained-layout fallback orchestration.

use super::*;

impl ConsolePanel {
    /// Render the real Reference list/detail subdivision after the unchanged header controls.
    /// This boundary also supports a genuinely constrained remaining pane without new docking.
    pub fn show_remaining(
        &mut self,
        ui: &mut Ui,
        console: &mut Console,
        remaining_rect: egui::Rect,
    ) {
        self.reconcile(console);
        self.rows.clear();
        self.controls.retain(|control| {
            control["role"]
                .as_str()
                .is_some_and(|role| !role.starts_with("console.row.") && role != "console.detail")
        });
        let (layout, valid_layout) = match self.detail.layout(remaining_rect) {
            Ok(layout) => (layout, true),
            Err(error) => {
                ui.response().on_hover_text(error);
                let Some(layout) = self.detail.snapshot().layout else {
                    self.paint_clip = None;
                    self.detail_scroll = None;
                    self.scroll_identity = None;
                    return;
                };
                (layout, false)
            }
        };
        // A rejected measurement cannot replace the last valid presentation.
        // Its retained regions still obey the current actual pane's paint clip.
        let clip = if valid_layout {
            remaining_rect.intersect(ui.clip_rect())
        } else {
            ui.clip_rect()
        };
        self.paint_clip = Some(clip);
        let lifetime = self.lifetime().map(ConsoleLifetime::value).unwrap_or(0);
        self.show_record_list(ui, console, layout.list_rect, clip, lifetime);
        self.show_detail(
            ui,
            console,
            layout.detail_rect,
            clip,
            lifetime,
            valid_layout,
        );
    }
}
