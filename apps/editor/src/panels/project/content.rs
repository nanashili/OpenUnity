//! Current-directory filtering, breadcrumb and virtualized asset grid.

use super::*;

impl ProjectBrowserPanel {
    pub(super) fn show_reference_content<'a>(
        &mut self,
        ui: &mut egui::Ui,
        project: &'a Project,
        nonempty: &std::collections::BTreeSet<&Path>,
        content_rect: egui::Rect,
        frame: &mut ReferenceFrame,
    ) -> Vec<&'a ProjectEntry> {
        let query = self.filter.to_lowercase();
        let entries: Vec<_> = project
            .entries()
            .iter()
            .filter(|e| {
                let parent = e.relative_path.parent().unwrap_or_else(|| Path::new(""));
                parent == self.directory
                    && e.relative_path
                        .file_name()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .to_lowercase()
                        .contains(&query)
            })
            .collect();
        let mut content = ui.new_child(
            egui::UiBuilder::new()
                .id_salt("project.reference.content")
                .max_rect(content_rect.shrink2(egui::vec2(4.0, 0.0)))
                .layout(egui::Layout::top_down(egui::Align::Min)),
        );
        content.set_clip_rect(content_rect.intersect(ui.clip_rect()));
        self.show_content_heading(&mut content, &query, entries.is_empty());
        self.show_asset_grid(&mut content, &entries, nonempty, frame);
        entries
    }

    fn show_content_heading(&self, content: &mut egui::Ui, query: &str, entries_empty: bool) {
        let breadcrumb = if self.directory.as_os_str().is_empty() {
            "Assets".to_owned()
        } else {
            format!("Assets / {}", self.directory.display())
        };
        content
            .add_sized(
                [content.available_width(), 18.0],
                egui::Label::new(&breadcrumb).truncate(),
            )
            .on_hover_text(&breadcrumb);
        if entries_empty {
            content.weak(if query.is_empty() {
                "This directory contains no discovered entries."
            } else {
                "No entries match this search."
            });
        }
    }

    fn show_asset_grid(
        &mut self,
        content: &mut egui::Ui,
        entries: &[&ProjectEntry],
        nonempty: &std::collections::BTreeSet<&Path>,
        frame: &mut ReferenceFrame,
    ) {
        let grid_columns = ((content.available_width() + 5.0) / 81.0).floor().max(1.0) as usize;
        let grid_rows = entries.len().div_ceil(grid_columns);
        content.spacing_mut().item_spacing.y = 5.0;
        egui::ScrollArea::vertical()
            .id_salt("project.reference.assets")
            .show_rows(content, 78.0, grid_rows, |ui, range| {
                for row in range {
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing = egui::vec2(5.0, 5.0);
                        let start = row * grid_columns;
                        let end = (start + grid_columns).min(entries.len());
                        for entry in &entries[start..end] {
                            self.show_asset_entry(ui, entry, nonempty, frame);
                        }
                    });
                }
            });
    }
}
