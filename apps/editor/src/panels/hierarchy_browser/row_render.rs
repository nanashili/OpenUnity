use crate::appearance;
use eframe::egui::{self, Response, Ui};

use super::{CachedRow, snapshot::RowGeometry};

const DISCLOSURE_RESERVATION: f32 = 12.0;

pub(super) struct RowRender {
    pub(super) geometry: RowGeometry,
    pub(super) control: serde_json::Value,
    pub(super) action: Option<(openunity_editor_core::scene::EntityId, bool)>,
    pub(super) branch_toggle: Option<usize>,
}

struct RowLayout {
    rect: egui::Rect,
    response: Response,
    indent: f32,
    indent_anchor: f32,
    disclosure_rect: Option<egui::Rect>,
}

struct RowContent {
    icon_rect: egui::Rect,
    label_origin: egui::Pos2,
    label_rect: egui::Rect,
}

pub(super) fn render_row(
    ui: &mut Ui,
    row: &CachedRow,
    row_index: usize,
    selected: bool,
    has_children: bool,
    expanded: bool,
) -> RowRender {
    let layout = allocate_row(ui, row, selected, has_children);
    let branch_toggle = show_disclosure(ui, row_index, &layout, expanded);
    let content = paint_row_content(ui, row, &layout);
    let geometry = package_row_geometry(row_index, &layout.response, &layout, content);
    let response = attach_row_tooltip(layout.response, row);
    let action = row_action(&response, row, branch_toggle.is_some());
    let control = appearance::control(
        format!("hierarchy.entity.{}", row.id.0),
        &response,
        selected,
    );

    RowRender {
        geometry,
        control,
        action,
        branch_toggle,
    }
}

fn allocate_row(ui: &mut Ui, row: &CachedRow, selected: bool, has_children: bool) -> RowLayout {
    let row_height = appearance::TREE_LINE_HEIGHT_FALLBACK;
    ui.spacing_mut().interact_size.y = row_height;

    let (rect, _) = ui.allocate_exact_size(
        egui::vec2(ui.available_width().max(180.0), row_height),
        egui::Sense::hover(),
    );
    let response = ui.put(rect, egui::Button::new("").selected(selected).frame(false));
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

fn show_disclosure(
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

fn paint_row_content(ui: &Ui, row: &CachedRow, layout: &RowLayout) -> RowContent {
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

fn attach_row_tooltip(response: Response, row: &CachedRow) -> Response {
    response.on_hover_ui(|ui| {
        ui.label(format!(
            "{} · editor ID {}{}",
            row.name,
            row.id.0,
            if row.active { "" } else { " · inactive" }
        ));
    })
}

fn row_action(
    response: &Response,
    row: &CachedRow,
    disclosure_clicked: bool,
) -> Option<(openunity_editor_core::scene::EntityId, bool)> {
    if !response.clicked() || disclosure_clicked {
        return None;
    }

    let toggle_selection = response
        .ctx
        .input(|input| input.modifiers.command || input.modifiers.ctrl);
    Some((row.id, toggle_selection))
}

fn package_row_geometry(
    row_index: usize,
    response: &Response,
    layout: &RowLayout,
    content: RowContent,
) -> RowGeometry {
    RowGeometry {
        row_index,
        widget_id: response.id.value(),
        row_rect: response.rect,
        indent_anchor: egui::pos2(layout.indent_anchor, layout.rect.center().y),
        indent_without_icon_padding: layout.indent,
        disclosure_reservation: DISCLOSURE_RESERVATION,
        disclosure_rect: layout.disclosure_rect,
        icon_rect: content.icon_rect,
        label_origin: content.label_origin,
        label_rect: content.label_rect,
        row_height: response.rect.height(),
    }
}

pub(super) fn draw_disclosure_glyph(ui: &Ui, rect: egui::Rect, expanded: bool) {
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
