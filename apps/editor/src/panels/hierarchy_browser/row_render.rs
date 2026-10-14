mod layout;

use crate::appearance;
use eframe::egui::{self, Response, Ui};

use super::{BrowserAction, CachedRow, sibling_controls, snapshot::RowGeometry};
use layout::{
    DISCLOSURE_RESERVATION, RowContent, RowLayout, allocate_row, paint_row_content, show_disclosure,
};

pub(super) use layout::draw_disclosure_glyph;

pub(super) struct RowRender {
    pub(super) geometry: RowGeometry,
    pub(super) control: serde_json::Value,
    pub(super) action: Option<BrowserAction>,
    pub(super) branch_toggle: Option<usize>,
}

pub(super) struct SiblingDragContext<'a> {
    pub(super) enabled: bool,
    pub(super) order: &'a [openunity_editor_core::scene::EntityId],
}

pub(super) fn render_row(
    ui: &mut Ui,
    row: &CachedRow,
    row_index: usize,
    selected: bool,
    has_children: bool,
    expanded: bool,
    sibling_drag: SiblingDragContext<'_>,
) -> RowRender {
    let layout = allocate_row(ui, row, selected, has_children, sibling_drag.enabled);
    let branch_toggle = show_disclosure(ui, row_index, &layout, expanded);
    let content = paint_row_content(ui, row, &layout);
    let sibling_action = sibling_controls::render(ui, row, layout.rect);
    let geometry = package_row_geometry(row_index, &layout.response, &layout, content);
    let response = attach_row_tooltip(layout.response, row);
    let sibling_drop = super::super::sibling_drag::observe_sibling_row(
        &response,
        row.id,
        row.root_position.or(row.sibling_position),
        sibling_drag.order,
        sibling_drag.enabled,
    );
    let action = sibling_drop
        .operation
        .map(|(dragged_id, index)| BrowserAction::SetSiblingIndex(dragged_id, index))
        .or(sibling_action)
        .or_else(|| {
            (!sibling_drop.completed)
                .then(|| row_action(&response, row, branch_toggle.is_some()))
                .flatten()
        });
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
) -> Option<BrowserAction> {
    if !response.clicked() || disclosure_clicked {
        return None;
    }

    let toggle_selection = response
        .ctx
        .input(|input| input.modifiers.command || input.modifiers.ctrl);
    Some(BrowserAction::Select(row.id, toggle_selection))
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
