//! Compact searchable tree over actual document identities.

use crate::appearance;
use eframe::egui::{self, Ui};
use openunity_editor_core::scene::EntityId;
use std::collections::{BTreeMap, BTreeSet};

pub(crate) struct Row {
    pub id: EntityId,
    pub parent: Option<EntityId>,
    pub name: String,
    pub depth: usize,
    pub active: bool,
    pub selected: bool,
}

/// Search and rendered tree receipt are shell state, separate from documents.
#[derive(Default)]
pub struct HierarchyBrowser {
    query: String,
    snapshot: serde_json::Value,
    controls: Vec<serde_json::Value>,
}

impl HierarchyBrowser {
    /// Live search, visible canonical identities, and actual control rectangles.
    pub fn snapshot(&self) -> serde_json::Value {
        self.snapshot.clone()
    }

    pub(crate) fn search(&mut self, ui: &mut Ui) {
        self.controls.clear();
        let (search_icon, _) = ui.allocate_exact_size(egui::vec2(8.0, 18.0), egui::Sense::hover());
        let response = ui.add_sized(
            [ui.available_width().max(20.0) - 22.0, 18.0],
            egui::TextEdit::singleline(&mut self.query).hint_text("Search hierarchy"),
        );
        crate::resources::paint(
            ui,
            "search",
            "hierarchy.search",
            response.id,
            egui::Rect::from_center_size(search_icon.center(), egui::vec2(8.0, 8.0)),
        );
        self.controls
            .push(appearance::control("hierarchy.search", &response, false));
        let clear =
            crate::resources::search_cancel_button(ui, !self.query.is_empty(), "hierarchy.clear");
        self.controls
            .push(appearance::control("hierarchy.clear", &clear, false));
        if clear.clicked() {
            self.query.clear();
        }
    }

    pub(crate) fn tree(
        &mut self,
        ui: &mut Ui,
        scene_name: &str,
        rows: &[Row],
    ) -> Vec<(EntityId, bool)> {
        let query = self.query.to_lowercase();
        let parents: BTreeMap<_, _> = rows.iter().map(|r| (r.id, r.parent)).collect();
        let branches: BTreeSet<_> = rows.iter().filter_map(|r| r.parent).collect();
        let mut included = BTreeSet::new();
        for row in rows
            .iter()
            .filter(|r| r.name.to_lowercase().contains(&query))
        {
            let mut id = Some(row.id);
            while let Some(current) = id {
                if !included.insert(current) {
                    break;
                }
                id = parents.get(&current).copied().flatten();
            }
        }
        let visible: Vec<_> = rows.iter().filter(|r| included.contains(&r.id)).collect();
        let header =
            egui::Rect::from_min_size(ui.cursor().min, egui::vec2(ui.available_width(), 18.0));
        ui.painter().rect_filled(header, 0.0, appearance::BAR);
        ui.add_sized([ui.available_width(), 18.0], egui::Label::new("Name"));
        ui.horizontal(|ui| {
            let (rect, _) = ui.allocate_exact_size(egui::vec2(12.0, 12.0), egui::Sense::hover());
            disclosure(ui, rect);
            ui.strong(scene_name)
                .on_hover_text("Actual active document");
        });
        // This custom row renderer has no explicit style-resolved height.
        let row_height = appearance::TREE_LINE_HEIGHT_FALLBACK;
        let mut rendered_rows = Vec::new();
        let mut geometry_truncated = false;
        let body = ui.available_rect_before_wrap();
        let stripe_count = (body.height() / row_height).ceil().max(0.0) as usize;
        for row in 0..stripe_count {
            let rect = egui::Rect::from_min_size(
                body.min + egui::vec2(0.0, row as f32 * row_height),
                egui::vec2(body.width(), row_height),
            )
            .intersect(body);
            ui.painter().rect_filled(
                rect,
                0.0,
                if row % 2 == 0 {
                    appearance::PANEL
                } else {
                    appearance::STRIPE
                },
            );
        }
        let mut actions = Vec::new();
        if visible.is_empty() {
            ui.weak(if rows.is_empty() {
                "Scene is empty."
            } else {
                "No entities match this search."
            });
        }
        ui.spacing_mut().item_spacing.y = 0.0;
        egui::ScrollArea::both()
            .id_salt("reference.hierarchy.rows")
            .show_rows(ui, row_height, visible.len(), |ui, range| {
                for row in &visible[range] {
                    ui.push_id(row.id.0, |ui| {
                        ui.spacing_mut().interact_size.y = row_height;
                        let (rect, _) = ui.allocate_exact_size(
                            egui::vec2(ui.available_width().max(180.0), row_height),
                            egui::Sense::hover(),
                        );
                        let response = ui.put(
                            rect,
                            egui::Button::new("").selected(row.selected).frame(false),
                        );
                        response.widget_info(|| {
                            egui::WidgetInfo::selected(
                                egui::WidgetType::SelectableLabel,
                                ui.is_enabled(),
                                row.selected,
                                row.name.as_str(),
                            )
                        });
                        let indent = appearance::tree_indent(row.depth);
                        let x = rect.left() + indent;
                        // Keep the existing disclosure reservation separate from
                        // indentation; its width is outside the metrics contract.
                        let disclosure_reservation = 12.0;
                        let mut disclosure_rect_trace = None;
                        let has_children = branches.contains(&row.id);
                        if has_children {
                            let disclosure_rect = egui::Rect::from_center_size(
                                egui::pos2(x + 5.0, rect.center().y),
                                egui::vec2(13.0, 13.0),
                            );
                            disclosure_rect_trace = Some(appearance::rect_json(disclosure_rect));
                            if !crate::resources::paint(
                                ui,
                                "disclosure_open",
                                "hierarchy.disclosure",
                                response.id,
                                disclosure_rect,
                            ) {
                                disclosure(ui, disclosure_rect);
                            }
                        }
                        let icon = egui::Rect::from_min_size(
                            egui::pos2(x + disclosure_reservation, rect.center().y - appearance::TREE_ICON_WIDTH * 0.5),
                            egui::vec2(appearance::TREE_ICON_WIDTH, appearance::TREE_ICON_WIDTH),
                        );
                        if !crate::resources::paint(
                            ui,
                            "game_object",
                            "hierarchy.entity",
                            response.id,
                            icon,
                        ) {
                            ui.painter().rect_stroke(
                                icon.shrink(4.0),
                                0.0,
                                egui::Stroke::new(1.0, appearance::MUTED),
                                egui::StrokeKind::Inside,
                            );
                        }
                        let label_origin = egui::pos2(icon.right() + appearance::TREE_ICON_LABEL_GAP, rect.center().y);
                        let label_rect = ui.painter().text(
                            label_origin,
                            egui::Align2::LEFT_CENTER,
                            &row.name,
                            egui::FontId::proportional(12.0),
                            if row.active {
                                appearance::TEXT
                            } else {
                                appearance::MUTED
                            },
                        );
                        if rendered_rows.len() < 4096 {
                            rendered_rows.push(serde_json::json!({
                                "role":format!("hierarchy.entity.{}",row.id.0),"widget_id":response.id.value(),
                                "id":row.id.0,"label":row.name,"depth":row.depth,"row_rect":appearance::rect_json(response.rect),
                                "indent_anchor":[x,rect.center().y],"indent_without_icon_padding":indent,
                                "disclosure_reservation":disclosure_reservation,"disclosure_rect":disclosure_rect_trace,
                                "icon_rect":appearance::rect_json(icon),"label_origin":[label_origin.x,label_origin.y],
                                "label_rect":appearance::rect_json(label_rect),"row_height":response.rect.height(),
                                "style_resolved_line_height":null,"row_height_source":"fallback"
                            }));
                        } else { geometry_truncated = true; }
                        self.controls.push(appearance::control(
                            format!("hierarchy.entity.{}", row.id.0),
                            &response,
                            row.selected,
                        ));
                        if response.clicked() {
                            actions.push((
                                row.id,
                                ui.input(|i| i.modifiers.command || i.modifiers.ctrl),
                            ));
                        }
                        response.on_hover_text(format!(
                            "{} · editor ID {}{}",
                            row.name,
                            row.id.0,
                            if row.active { "" } else { " · inactive" }
                        ));
                    });
                }
            });
        self.snapshot = serde_json::json!({"query":self.query,"visible_ids":visible.iter().map(|r|r.id.0).collect::<Vec<_>>(),
            "visible_rows":visible.iter().map(|r|serde_json::json!({"id":r.id.0,"name":r.name,"depth":r.depth,"selected":r.selected})).collect::<Vec<_>>(),"controls":self.controls,"content_rect":appearance::rect_json(body),"rendered_rows":rendered_rows,"geometry_truncated":geometry_truncated});
        actions
    }
}

fn disclosure(ui: &Ui, rect: egui::Rect) {
    let r = rect.shrink2(egui::vec2(2.0, 3.0));
    ui.painter().add(egui::Shape::convex_polygon(
        vec![r.left_top(), r.right_top(), r.center_bottom()],
        appearance::MUTED,
        egui::Stroke::NONE,
    ));
}
