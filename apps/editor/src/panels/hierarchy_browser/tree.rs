use crate::appearance;
use eframe::egui::{self, Ui};
use openunity_editor_core::scene::EntityId;

use super::snapshot::MAX_RENDERED_GEOMETRY;
use super::{BrowserAction, HierarchyBrowser, row_render};

impl HierarchyBrowser {
    pub(crate) fn tree(
        &mut self,
        ui: &mut Ui,
        scene_name: &str,
        selected: impl IntoIterator<Item = EntityId>,
    ) -> Vec<BrowserAction> {
        self.update_selected_ids(selected);

        draw_column_header(ui);
        self.draw_scene_header(ui, scene_name);
        self.ensure_visible_indices();

        let row_height = appearance::TREE_LINE_HEIGHT_FALLBACK;
        let body = ui.available_rect_before_wrap();
        draw_row_stripes(ui, body, row_height);
        self.content_rect = Some(body);

        let source_is_empty = self
            .source
            .as_ref()
            .is_none_or(|source| source.rows.is_empty());
        let visible_count = self.visible_indices.as_ref().map_or(0, Vec::len);
        if visible_count == 0 {
            show_empty_state(ui, source_is_empty, !self.lowercase_query.is_empty());
        }

        let mut actions = Vec::new();
        let mut branch_toggles = Vec::new();
        self.render_viewport_rows(ui, row_height, &mut actions, &mut branch_toggles);

        for row_index in branch_toggles {
            self.toggle_branch_expansion(row_index);
        }
        self.ensure_visible_indices();
        actions
    }

    fn update_selected_ids(&mut self, selected: impl IntoIterator<Item = EntityId>) {
        self.selected_ids.clear();
        self.selected_ids.extend(selected);
        self.selected_ids.sort_unstable();
        self.selected_ids.dedup();
    }

    fn draw_scene_header(&mut self, ui: &mut Ui, scene_name: &str) {
        ui.horizontal(|ui| {
            let (rect, _) = ui.allocate_exact_size(egui::vec2(12.0, 12.0), egui::Sense::hover());
            let expanded = !self.lowercase_query.is_empty() || self.scene_expanded;
            let response = ui
                .interact(
                    rect,
                    ui.id().with("hierarchy.scene.disclosure"),
                    egui::Sense::click(),
                )
                .on_hover_text(if expanded {
                    "Collapse scene"
                } else {
                    "Expand scene"
                });
            row_render::draw_disclosure_glyph(ui, rect, expanded);
            if response.clicked() {
                self.toggle_scene_expansion();
            }

            ui.strong(scene_name)
                .on_hover_text("Actual active document");
        });
    }

    fn render_viewport_rows(
        &mut self,
        ui: &mut Ui,
        row_height: f32,
        actions: &mut Vec<BrowserAction>,
        branch_toggles: &mut Vec<usize>,
    ) {
        let Some(source) = &self.source else {
            return;
        };
        let Some(visible_indices) = &self.visible_indices else {
            return;
        };

        self.row_geometry.clear();
        self.row_controls.clear();
        self.geometry_truncated = false;

        let source_rows = &source.rows;
        let selected_ids = &self.selected_ids;
        let collapsed_rows = &self.collapsed_rows;
        let query_active = !self.lowercase_query.is_empty();
        let geometry = &mut self.row_geometry;
        let controls = &mut self.row_controls;
        let geometry_truncated = &mut self.geometry_truncated;

        ui.spacing_mut().item_spacing.y = 0.0;
        egui::ScrollArea::both()
            .id_salt("reference.hierarchy.rows")
            .show_rows(ui, row_height, visible_indices.len(), |ui, range| {
                for visible_position in range {
                    let row_index = visible_indices[visible_position];
                    let row = &source_rows[row_index];
                    let selected = selected_ids.binary_search(&row.id).is_ok();
                    let has_children = row.has_children;
                    let collapsed = collapsed_rows.get(row_index).copied().unwrap_or(false);
                    let expanded = query_active || !collapsed;

                    let rendered = ui
                        .push_id(row.id.0, |ui| {
                            row_render::render_row(
                                ui,
                                row,
                                row_index,
                                selected,
                                has_children,
                                expanded,
                            )
                        })
                        .inner;

                    if geometry.len() < MAX_RENDERED_GEOMETRY {
                        geometry.push(rendered.geometry);
                        controls.push(rendered.control);
                    } else {
                        *geometry_truncated = true;
                    }

                    if let Some(action) = rendered.action {
                        actions.push(action);
                    }
                    if let Some(row_index) = rendered.branch_toggle {
                        branch_toggles.push(row_index);
                    }
                }
            });
    }
}

fn draw_column_header(ui: &mut Ui) {
    let header = egui::Rect::from_min_size(ui.cursor().min, egui::vec2(ui.available_width(), 18.0));
    ui.painter().rect_filled(header, 0.0, appearance::BAR);
    ui.add_sized([ui.available_width(), 18.0], egui::Label::new("Name"));
}

fn draw_row_stripes(ui: &Ui, body: egui::Rect, row_height: f32) {
    let stripe_count = (body.height() / row_height).ceil().max(0.0) as usize;
    for index in 0..stripe_count {
        let rect = egui::Rect::from_min_size(
            body.min + egui::vec2(0.0, index as f32 * row_height),
            egui::vec2(body.width(), row_height),
        )
        .intersect(body);
        let color = if index % 2 == 0 {
            appearance::PANEL
        } else {
            appearance::STRIPE
        };
        ui.painter().rect_filled(rect, 0.0, color);
    }
}

fn show_empty_state(ui: &mut Ui, source_is_empty: bool, query_active: bool) {
    if source_is_empty {
        ui.weak("Scene is empty.");
    } else if query_active {
        ui.weak("No entities match this search.");
    }
}
