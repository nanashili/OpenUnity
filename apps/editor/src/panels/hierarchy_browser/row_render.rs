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
    pub(super) structural_edits_enabled: bool,
    pub(super) order: &'a [openunity_editor_core::scene::EntityId],
}

pub(super) struct RowContext<'a> {
    pub(super) row_index: usize,
    pub(super) selected: bool,
    pub(super) visibility_state: crate::scene_visibility::VisibilityRowState,
    pub(super) authored_scene: Option<(
        &'a openunity_editor_core::scene::Scene,
        &'a mut crate::scene_visibility::SceneVisibility,
    )>,
    pub(super) has_children: bool,
    pub(super) expanded: bool,
    pub(super) sibling_drag: SiblingDragContext<'a>,
}

pub(super) fn render_row(ui: &mut Ui, row: &CachedRow, context: RowContext<'_>) -> RowRender {
    let RowContext {
        row_index,
        selected,
        visibility_state,
        authored_scene,
        has_children,
        expanded,
        sibling_drag,
    } = context;
    let layout = allocate_row(ui, row, selected, has_children, sibling_drag.enabled);
    let branch_toggle = show_disclosure(ui, row_index, &layout, expanded);
    let content = paint_row_content(ui, row, &layout);
    let visibility_clicked = authored_scene.is_some_and(|(scene, visibility)| {
        render_visibility_control(ui, row, &layout, visibility_state, scene, visibility)
    });
    let sibling_action = sibling_drag
        .structural_edits_enabled
        .then(|| sibling_controls::render(ui, row, layout.rect))
        .flatten();
    let geometry = package_row_geometry(row_index, &layout.response, &layout, content);
    let response = attach_row_tooltip(layout.response, row);
    let sibling_drop = super::super::sibling_drag::observe_sibling_row(
        &response,
        row.id,
        row.root_position.or(row.sibling_position),
        sibling_drag.order,
        sibling_drag.enabled && !visibility_clicked,
    );
    let action = sibling_drop
        .operation
        .map(|operation| match operation {
            super::super::sibling_drag::SiblingDropOperation::Reorder { id, sibling_index } => {
                BrowserAction::SetSiblingIndex(id, sibling_index)
            }
            super::super::sibling_drag::SiblingDropOperation::Reparent { id, parent } => {
                BrowserAction::Reparent(id, parent)
            }
        })
        .or(sibling_action)
        .or_else(|| {
            (!sibling_drop.completed && !visibility_clicked)
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

fn render_visibility_control(
    ui: &mut Ui,
    row: &CachedRow,
    layout: &RowLayout,
    state: crate::scene_visibility::VisibilityRowState,
    scene: &openunity_editor_core::scene::Scene,
    visibility: &mut crate::scene_visibility::SceneVisibility,
) -> bool {
    let (glyph, description) = state.control_label();
    let rect = egui::Rect::from_center_size(
        egui::pos2(layout.rect.right() - 12.0, layout.rect.center().y),
        egui::vec2(20.0, layout.rect.height()),
    );
    let response = ui
        .put(rect, egui::Button::new(glyph).frame(false))
        .on_hover_text(format!(
            "{description}. Alt-click changes only this entity."
        ));

    if response.clicked() {
        let recursive = !ui.input(|input| input.modifiers.alt);
        visibility.toggle(scene, row.id, recursive);
    }

    response.hovered()
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
