//! Reference record-list clipping, virtualization and actual row selection.

use super::*;
use openunity_editor_core::console::Record;

impl ConsolePanel {
    pub(super) fn show_record_list(
        &mut self,
        ui: &mut Ui,
        console: &Console,
        list_rect: egui::Rect,
        clip: egui::Rect,
        lifetime: u64,
    ) {
        let list_clip = list_rect.intersect(clip);
        if list_clip.width() > 0.0 && list_clip.height() > 0.0 {
            let mut list_ui = ui.new_child(
                egui::UiBuilder::new()
                    .id_salt(("console.list", lifetime))
                    .max_rect(list_rect),
            );
            list_ui.set_clip_rect(list_clip);
            let severities = self.severities();
            let filter = self.filter.clone();
            let records: Vec<_> = console.filtered(&severities, &filter).collect();
            if records.is_empty() {
                list_ui.label(if console.records().len() == 0 {
                    "No editor records."
                } else {
                    "No records match the current filters."
                });
            } else {
                self.show_record_rows(
                    &mut list_ui,
                    &records,
                    console,
                    &severities,
                    lifetime,
                    list_rect.height(),
                );
            }
        }
    }

    fn show_record_rows(
        &mut self,
        ui: &mut Ui,
        records: &[&Record],
        console: &Console,
        severities: &[Severity],
        lifetime: u64,
        list_height: f32,
    ) {
        let row_height = ui.text_style_height(&TextStyle::Body);
        ui.spacing_mut().interact_size.y = row_height;
        ui.spacing_mut().button_padding.y = 0.0;
        ScrollArea::vertical()
            .id_salt(("console_records", lifetime))
            .auto_shrink([false, false])
            .min_scrolled_height(0.0)
            .max_height(list_height)
            .show_rows(ui, row_height, records.len(), |ui, range| {
                for record in &records[range] {
                    self.show_record_row(ui, record, console, severities, lifetime, row_height);
                }
            });
    }

    fn show_record_row(
        &mut self,
        ui: &mut Ui,
        record: &Record,
        console: &Console,
        severities: &[Severity],
        lifetime: u64,
        row_height: f32,
    ) {
        let Some(identity) = self.identity(record.sequence) else {
            return;
        };
        let selected = self.detail.snapshot().selected == Some(identity);
        let text = format!(
            "#{} [{}] {}",
            record.sequence,
            severity_label(record.severity),
            record.message
        );
        let response = ui
            .push_id(("console.row", lifetime, record.sequence), |ui| {
                ui.add_sized(
                    [ui.available_width(), row_height],
                    egui::Button::new(&text)
                        .truncate()
                        .selected(selected)
                        .fill(if selected {
                            crate::appearance::SELECTION
                        } else {
                            egui::Color32::TRANSPARENT
                        })
                        .stroke(egui::Stroke::NONE),
                )
            })
            .inner;
        response.widget_info(|| {
            egui::WidgetInfo::selected(
                egui::WidgetType::SelectableLabel,
                response.enabled(),
                selected,
                &text,
            )
        });
        let response = response.on_hover_text(&record.message);
        self.record_control(
            &format!("console.row.{}", record.sequence),
            &response,
            selected,
        );
        self.rows.push(serde_json::json!({
            "sequence": record.sequence,
            "rect": crate::appearance::rect_json(response.rect),
            "widget_id": response.id.value(),
            "selected": selected,
        }));
        if response.clicked() {
            let _ = self
                .detail
                .select(identity, console, severities, &self.filter);
            ui.ctx().request_repaint();
        }
    }
}
