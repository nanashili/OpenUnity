use crate::appearance;
use eframe::egui::{Pos2, Rect};
use serde_json::{Value, json};

use super::{CachedRow, HierarchyBrowser};

pub(super) const MAX_RENDERED_GEOMETRY: usize = 4096;

pub(super) struct RowGeometry {
    pub(super) row_index: usize,
    pub(super) widget_id: u64,
    pub(super) row_rect: Rect,
    pub(super) indent_anchor: Pos2,
    pub(super) indent_without_icon_padding: f32,
    pub(super) disclosure_reservation: f32,
    pub(super) disclosure_rect: Option<Rect>,
    pub(super) icon_rect: Rect,
    pub(super) label_origin: Pos2,
    pub(super) label_rect: Rect,
    pub(super) row_height: f32,
}

impl HierarchyBrowser {
    /// Live search, visible canonical identities, and actual control rectangles.
    pub fn snapshot(&self) -> Value {
        if self.visible_indices.is_none() {
            return Value::Null;
        }

        self.build_snapshot()
    }

    fn build_snapshot(&self) -> Value {
        let visible_indices = self.visible_indices.as_deref().unwrap_or_default();
        let rows = self
            .source
            .as_ref()
            .map(|source| source.rows.as_slice())
            .unwrap_or_default();

        let visible_ids = visible_indices
            .iter()
            .filter_map(|index| rows.get(*index))
            .map(|row| row.id.0)
            .collect::<Vec<_>>();
        let visible_rows = visible_indices
            .iter()
            .filter_map(|index| rows.get(*index))
            .map(|row| visible_row_snapshot(row, &self.selected_ids))
            .collect::<Vec<_>>();
        let rendered_rows = self
            .row_geometry
            .iter()
            .filter_map(|geometry| {
                rows.get(geometry.row_index)
                    .map(|row| rendered_row_snapshot(row, geometry))
            })
            .collect::<Vec<_>>();
        let controls = self
            .search_controls
            .iter()
            .chain(&self.row_controls)
            .cloned()
            .collect::<Vec<_>>();
        let content_rect = self.content_rect.map(appearance::rect_json);

        json!({
            "query": self.query,
            "visible_ids": visible_ids,
            "visible_rows": visible_rows,
            "controls": controls,
            "content_rect": content_rect,
            "rendered_rows": rendered_rows,
            "geometry_truncated": self.geometry_truncated,
        })
    }
}

fn visible_row_snapshot(
    row: &CachedRow,
    selected_ids: &[openunity_editor_core::scene::EntityId],
) -> Value {
    json!({
        "id": row.id.0,
        "name": row.name,
        "depth": row.depth,
        "selected": selected_ids.binary_search(&row.id).is_ok(),
    })
}

fn rendered_row_snapshot(row: &CachedRow, geometry: &RowGeometry) -> Value {
    let disclosure_rect = geometry.disclosure_rect.map(appearance::rect_json);
    json!({
        "role": format!("hierarchy.entity.{}", row.id.0),
        "widget_id": geometry.widget_id,
        "id": row.id.0,
        "label": row.name,
        "depth": row.depth,
        "row_rect": appearance::rect_json(geometry.row_rect),
        "indent_anchor": [geometry.indent_anchor.x, geometry.indent_anchor.y],
        "indent_without_icon_padding": geometry.indent_without_icon_padding,
        "disclosure_reservation": geometry.disclosure_reservation,
        "disclosure_rect": disclosure_rect,
        "icon_rect": appearance::rect_json(geometry.icon_rect),
        "label_origin": [geometry.label_origin.x, geometry.label_origin.y],
        "label_rect": appearance::rect_json(geometry.label_rect),
        "row_height": geometry.row_height,
        "style_resolved_line_height": null,
        "row_height_source": "fallback",
    })
}
