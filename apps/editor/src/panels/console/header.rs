//! Reference severity, clear and message-filter controls.

use super::*;

impl ConsolePanel {
    pub(super) fn show_reference(&mut self, ui: &mut Ui, console: &mut Console) {
        self.controls.clear();
        self.severity_icons.clear();
        self.reconcile(console);
        let mut counts = [0usize; 3];
        for record in console.records() {
            counts[match record.severity {
                Severity::Info => 0,
                Severity::Warning => 1,
                Severity::Error => 2,
            }] += 1;
        }
        self.show_severity_controls(ui, console, counts);
        ui.horizontal(|ui| {
            let label = ui.label("Filter messages");
            let response = ui
                .add(TextEdit::singleline(&mut self.filter).hint_text("Search message text"))
                .labelled_by(label.id)
                .on_hover_text("Literal, case-sensitive message substring");
            self.record_control("console.filter", &response, false);
        });
        self.reconcile(console);
        ui.small(format!(
            "Showing {} of {} retained records · Dropped records: {}",
            self.visible_sequences.len(),
            console.records().len(),
            console.dropped_count()
        ));
        ui.separator();
        self.show_remaining(ui, console, ui.available_rect_before_wrap());
    }

    fn show_severity_controls(&mut self, ui: &mut Ui, console: &mut Console, counts: [usize; 3]) {
        ui.horizontal_wrapped(|ui| {
            let (response, icon) =
                Self::severity_checkbox(ui, &mut self.info, "Info", "console_info", counts[0]);
            self.record_control("console.severity.Info", &response, self.info);
            self.severity_icons.push(icon);
            let (response, icon) = Self::severity_checkbox(
                ui,
                &mut self.warning,
                "Warning",
                "console_warning",
                counts[1],
            );
            self.record_control("console.severity.Warning", &response, self.warning);
            self.severity_icons.push(icon);
            let (response, icon) =
                Self::severity_checkbox(ui, &mut self.error, "Error", "console_error", counts[2]);
            self.record_control("console.severity.Error", &response, self.error);
            self.severity_icons.push(icon);
            let response = ui.add_enabled(console.records().len() != 0, egui::Button::new("Clear"));
            self.record_control("console.clear", &response, false);
            if response.clicked() {
                console.clear();
                self.reconcile(console);
            }
        });
    }
}
