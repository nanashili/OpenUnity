//! Reference pane allocation and ordered browser presentation.

use super::*;

impl ReferenceLayout {
    pub(super) fn paint(ui: &mut egui::Ui) -> Self {
        let available = ui.available_rect_before_wrap();
        let footer_height = 18.0;
        let body = egui::Rect::from_min_max(
            available.min,
            egui::pos2(
                available.right(),
                (available.bottom() - footer_height).max(available.top() + 1.0),
            ),
        );
        let tree_width = (body.width() * 0.124)
            .clamp(140.0, 240.0)
            .min((body.width() - 90.0).max(1.0));
        let tree_rect = egui::Rect::from_min_size(body.min, egui::vec2(tree_width, body.height()));
        let content_rect =
            egui::Rect::from_min_max(egui::pos2(tree_rect.right() + 1.0, body.top()), body.max);
        ui.painter()
            .rect_filled(content_rect, 0.0, appearance::ASSETS);
        ui.painter().line_segment(
            [tree_rect.right_top(), tree_rect.right_bottom()],
            egui::Stroke::new(1.0, appearance::DIVIDER),
        );
        Self {
            available,
            body,
            tree_rect,
            content_rect,
        }
    }
}

impl ProjectBrowserPanel {
    pub(super) fn show_reference(&mut self, ui: &mut egui::Ui, project: Option<&Project>) {
        let mut frame = ReferenceFrame::new();
        self.reconcile_reference_project(project);

        self.show_reference_toolbar(ui, project, &mut frame);

        let layout = ReferenceLayout::paint(ui);
        let mut tree = ui.new_child(
            egui::UiBuilder::new()
                .id_salt(("project.reference.tree", &self.source_key))
                .max_rect(layout.tree_rect)
                .layout(egui::Layout::top_down(egui::Align::Min)),
        );
        tree.set_clip_rect(layout.tree_rect.intersect(ui.clip_rect()));

        let Some(project) = project else {
            Self::show_missing_project(ui, layout.content_rect);
            self.record_missing_project(&layout, frame);
            return;
        };

        let Some(source) = source::resolve(project, &self.source_key) else {
            self.record_missing_project(&layout, frame);
            return;
        };

        self.reconcile_directory(source);
        self.show_reference_tree(&mut tree, source, &mut frame);

        let entry_directory = self.directory.clone();
        let visible_indices =
            self.query_cache
                .reference_indices(project, source, &entry_directory, &self.filter);
        let entry_count = self.show_reference_content(
            ui,
            source,
            &visible_indices,
            &entry_directory,
            layout.content_rect,
            &mut frame,
        );

        self.show_reference_footer(ui, &layout, source, entry_count);
        self.record_reference(
            &layout,
            frame,
            project,
            source,
            &entry_directory,
            &visible_indices,
        );
    }

    fn show_missing_project(ui: &mut egui::Ui, content_rect: egui::Rect) {
        let mut content = ui.new_child(egui::UiBuilder::new().max_rect(content_rect.shrink(8.0)));
        content.label("No project is open.");
        content.small("File → Open Project to browse an existing Assets directory.");
    }

    fn show_reference_footer(
        &self,
        ui: &mut egui::Ui,
        layout: &ReferenceLayout,
        source: ProjectSource<'_>,
        entry_count: usize,
    ) {
        let footer = egui::Rect::from_min_max(
            egui::pos2(layout.body.left(), layout.body.bottom() + 1.0),
            layout.available.max,
        );
        ui.painter().rect_filled(footer, 0.0, appearance::BAR);
        let mut footer_ui = ui.new_child(egui::UiBuilder::new().max_rect(footer));
        let details = self.selected_path.as_ref().map_or_else(
            || format!("{} entries · read-only", entry_count),
            |path| {
                let logical_path = source::logical_path(source.key(), path);
                let kind = self.selected_path_kind.map(entry_kind).unwrap_or("entry");
                format!("{logical_path} ({kind}) · read-only · not imported")
            },
        );
        footer_ui
            .add(egui::Label::new(&details).truncate())
            .on_hover_text(details);
    }
}
