//! Legacy controls and retained-record rendering.

use super::*;
use openunity_editor_core::console::Record;

impl ConsolePanel {
    /// Displays retained records and applies clear directly to the supplied console.
    pub fn show(&mut self, ui: &mut Ui, console: &mut Console) {
        if crate::appearance::is_reference(ui) {
            self.show_reference(ui, console);
            return;
        }
        self.show_legacy(ui, console);
    }

    fn show_legacy(&mut self, ui: &mut Ui, console: &mut Console) {
        // Legacy renders none of the Reference record/detail controls. Retain
        // semantic selection and its offset for a later Reference frame.
        self.controls.clear();
        self.severity_icons.clear();
        self.rows.clear();
        self.paint_clip = None;
        self.detail_scroll = None;
        self.scroll_identity = None;
        let compact = crate::appearance::is_reference(ui);
        self.show_legacy_filters(ui, console);
        let severities: Vec<_> = [
            (self.info, Severity::Info),
            (self.warning, Severity::Warning),
            (self.error, Severity::Error),
        ]
        .into_iter()
        .filter_map(|(enabled, severity)| enabled.then_some(severity))
        .collect();
        let records: Vec<_> = console.filtered(&severities, &self.filter).collect();
        let summary = format!(
            "Showing {} of {} retained records · Dropped records: {}",
            records.len(),
            console.records().len(),
            console.dropped_count()
        );
        if compact {
            ui.small(summary);
        } else {
            ui.label(summary);
        }
        ui.separator();
        if records.is_empty() {
            ui.label(if console.records().len() == 0 {
                "No editor records."
            } else {
                "No records match the current filters."
            });
        }
        self.show_legacy_rows(ui, &records);
    }

    fn show_legacy_filters(&mut self, ui: &mut Ui, console: &mut Console) {
        ui.horizontal_wrapped(|ui| {
            ui.checkbox(&mut self.info, "Info");
            ui.checkbox(&mut self.warning, "Warning");
            ui.checkbox(&mut self.error, "Error");
            if ui
                .add_enabled(
                    console.records().len() != 0,
                    eframe::egui::Button::new("Clear"),
                )
                .clicked()
            {
                console.clear();
            }
        });
        ui.horizontal(|ui| {
            let label = ui.label("Filter messages");
            ui.add(TextEdit::singleline(&mut self.filter).hint_text("Search message text"))
                .labelled_by(label.id)
                .on_hover_text("Literal, case-sensitive message substring");
        });
    }

    fn show_legacy_rows(&self, ui: &mut Ui, records: &[&Record]) {
        // Console retention bounds this list to 1024 entries. Render only visible
        // fixed-height rows; a hovered row reveals its unmodified full message.
        let row_height = ui.text_style_height(&TextStyle::Body);
        ScrollArea::vertical().id_salt("console_records").show_rows(
            ui,
            row_height,
            records.len(),
            |ui, range| {
                for record in &records[range] {
                    let severity = match record.severity {
                        Severity::Info => "Info",
                        Severity::Warning => "Warning",
                        Severity::Error => "Error",
                    };
                    ui.push_id(record.sequence, |ui| {
                        ui.add(
                            Label::new(format!(
                                "#{} [{severity}] {}",
                                record.sequence, record.message
                            ))
                            .truncate(),
                        )
                        .on_hover_text(&record.message);
                    });
                }
            },
        );
    }
}
