//! Current-directory filtering, breadcrumb and virtualized asset grid.

use super::*;

impl ProjectBrowserPanel {
    pub(super) fn show_reference_content(
        &mut self,
        ui: &mut egui::Ui,
        project: &Project,
        visible_indices: &super::cache::CachedEntryIndices,
        directory: &Path,
        content_rect: egui::Rect,
        frame: &mut ReferenceFrame,
    ) -> usize {
        let query = self.filter.as_str();
        let indices = visible_indices.reference_slice(project, directory);

        let mut content = ui.new_child(
            egui::UiBuilder::new()
                .id_salt("project.reference.content")
                .max_rect(content_rect.shrink2(egui::vec2(4.0, 0.0)))
                .layout(egui::Layout::top_down(egui::Align::Min)),
        );
        content.set_clip_rect(content_rect.intersect(ui.clip_rect()));
        self.show_content_heading(&mut content, query, indices.is_empty());
        self.show_asset_grid(&mut content, project, indices, frame);
        indices.len()
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
        project: &Project,
        entry_indices: &[usize],
        frame: &mut ReferenceFrame,
    ) {
        let grid_columns = ((content.available_width() + 5.0) / 81.0).floor().max(1.0) as usize;
        let grid_rows = entry_indices.len().div_ceil(grid_columns);
        content.spacing_mut().item_spacing.y = 5.0;
        egui::ScrollArea::vertical()
            .id_salt("project.reference.assets")
            .show_rows(content, 78.0, grid_rows, |ui, range| {
                for row in range {
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing = egui::vec2(5.0, 5.0);
                        let start = row * grid_columns;
                        let end = (start + grid_columns).min(entry_indices.len());
                        for &entry_index in &entry_indices[start..end] {
                            let Some(entry) = project.entries().get(entry_index) else {
                                continue;
                            };
                            self.show_asset_entry(ui, project, entry, frame);
                        }
                    });
                }
            });
    }
}
