use crate::appearance;
use eframe::egui::{self, Response, Ui};

use super::super::CachedRow;

pub(super) const DISCLOSURE_RESERVATION: f32 = 12.0;

pub(super) struct RowLayout {
    pub(super) rect: egui::Rect,
    pub(super) response: Response,
    pub(super) indent: f32,
    pub(super) indent_anchor: f32,
    pub(super) disclosure_rect: Option<egui::Rect>,
}

pub(super) struct RowContent {
    pub(super) icon_rect: egui::Rect,
    pub(super) label_origin: egui::Pos2,
    pub(super) label_rect: egui::Rect,
}

pub(super) fn allocate_row(
    ui: &mut Ui,
    row: &CachedRow,
    selected: bool,
    has_children: bool,
    root_drag_enabled: bool,
) -> RowLayout {
    let row_height = appearance::TREE_LINE_HEIGHT_FALLBACK;
    ui.spacing_mut().interact_size.y = row_height;

    let (rect, _) = ui.allocate_exact_size(
        egui::vec2(ui.available_width().max(180.0), row_height),
        egui::Sense::hover(),
    );
    let button = egui::Button::new("").selected(selected).frame(false);
    let button = if row.root_position.is_some() && root_drag_enabled {
        button.sense(egui::Sense::click_and_drag())
    } else {
        button
    };
    let response = ui.put(rect, button);
    response.widget_info(|| {
        egui::WidgetInfo::selected(
            egui::WidgetType::SelectableLabel,
            ui.is_enabled(),
            selected,
            row.name.as_str(),
        )
    });

    let indent = appearance::tree_indent(row.depth);
    let indent_anchor = rect.left() + indent;
    let disclosure_rect = has_children.then(|| {
        egui::Rect::from_center_size(
            egui::pos2(indent_anchor + 5.0, rect.center().y),
            egui::vec2(13.0, 13.0),
        )
    });

    RowLayout {
        rect,
        response,
        indent,
        indent_anchor,
        disclosure_rect,
    }
}

pub(super) fn show_disclosure(
    ui: &mut Ui,
    row_index: usize,
    layout: &RowLayout,
    expanded: bool,
) -> Option<usize> {
    let disclosure_rect = layout.disclosure_rect?;
    let disclosure_response = ui
        .interact(
            disclosure_rect,
            layout.response.id.with("hierarchy.disclosure"),
            egui::Sense::click(),
        )
        .on_hover_text(if expanded {
            "Collapse branch"
        } else {
            "Expand branch"
        });

    let kind = if expanded {
        "disclosure_open"
    } else {
        "disclosure_closed"
    };
    if !crate::resources::paint(
        ui,
        kind,
        "hierarchy.disclosure",
        layout.response.id,
        disclosure_rect,
    ) {
        draw_disclosure_glyph(ui, disclosure_rect, expanded);
    }

    disclosure_response.clicked().then_some(row_index)
}

pub(super) fn paint_row_content(ui: &Ui, row: &CachedRow, layout: &RowLayout) -> RowContent {
    let icon_rect = egui::Rect::from_min_size(
        egui::pos2(
            layout.indent_anchor + DISCLOSURE_RESERVATION,
            layout.rect.center().y - appearance::TREE_ICON_WIDTH * 0.5,
        ),
        egui::vec2(appearance::TREE_ICON_WIDTH, appearance::TREE_ICON_WIDTH),
    );
    if !crate::resources::paint(
        ui,
        "game_object",
        "hierarchy.entity",
        layout.response.id,
        icon_rect,
    ) {
        ui.painter().rect_stroke(
            icon_rect.shrink(4.0),
            0.0,
            egui::Stroke::new(1.0, appearance::MUTED),
            egui::StrokeKind::Inside,
        );
    }

    let label_origin = egui::pos2(
        icon_rect.right() + appearance::TREE_ICON_LABEL_GAP,
        layout.rect.center().y,
    );
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

    RowContent {
        icon_rect,
        label_origin,
        label_rect,
    }
}

pub(crate) fn draw_disclosure_glyph(ui: &Ui, rect: egui::Rect, expanded: bool) {
    let shape = rect.shrink2(egui::vec2(2.0, 3.0));
    let points = if expanded {
        vec![shape.left_top(), shape.right_top(), shape.center_bottom()]
    } else {
        vec![shape.left_top(), shape.left_bottom(), shape.right_center()]
    };
    ui.painter().add(egui::Shape::convex_polygon(
        points,
        appearance::MUTED,
        egui::Stroke::NONE,
    ));
}
