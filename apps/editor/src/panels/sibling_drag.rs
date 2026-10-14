//! Shared sibling-row drag behavior for authored hierarchy presentations.

use eframe::egui::{self, Response};
use openunity_editor_core::scene::EntityId;

#[derive(Clone, Copy)]
struct SiblingDragPayload {
    id: EntityId,
}

pub(crate) struct SiblingDrop {
    pub operation: Option<SiblingDropOperation>,
    pub completed: bool,
}

pub(crate) enum SiblingDropOperation {
    Reorder { id: EntityId, sibling_index: usize },
    Reparent { id: EntityId, parent: EntityId },
}

pub(crate) fn observe_sibling_row(
    response: &Response,
    id: EntityId,
    sibling_position: Option<(usize, usize)>,
    sibling_order: &[EntityId],
    enabled: bool,
) -> SiblingDrop {
    let Some((sibling_index, _sibling_count)) = sibling_position.filter(|_| enabled) else {
        return SiblingDrop {
            operation: None,
            completed: false,
        };
    };

    response.dnd_set_drag_payload(SiblingDragPayload { id });
    let placement = drop_placement(response);
    paint_drop_feedback(response, id, sibling_order, placement);

    let payload = response.dnd_release_payload::<SiblingDragPayload>();
    let Some(payload) = payload else {
        return SiblingDrop {
            operation: None,
            completed: false,
        };
    };

    let operation = if payload.id == id {
        None
    } else if placement == DropPlacement::Parent {
        Some(SiblingDropOperation::Reparent {
            id: payload.id,
            parent: id,
        })
    } else if sibling_order.contains(&payload.id) {
        insertion_index(sibling_index, sibling_order, *payload, placement).map(|index| {
            SiblingDropOperation::Reorder {
                id: payload.id,
                sibling_index: index,
            }
        })
    } else {
        None
    };

    SiblingDrop {
        operation,
        completed: true,
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum DropPlacement {
    Before,
    Parent,
    After,
}

fn drop_placement(response: &Response) -> DropPlacement {
    let pointer_y = response
        .interact_pointer_pos()
        .map_or_else(|| response.rect.center().y, |pointer| pointer.y);
    let relative_y = (pointer_y - response.rect.top()) / response.rect.height().max(1.0);

    if relative_y < 0.3 {
        DropPlacement::Before
    } else if relative_y > 0.7 {
        DropPlacement::After
    } else {
        DropPlacement::Parent
    }
}

fn paint_drop_feedback(
    response: &Response,
    target_id: EntityId,
    sibling_order: &[EntityId],
    placement: DropPlacement,
) {
    let Some(payload) = response.dnd_hover_payload::<SiblingDragPayload>() else {
        return;
    };
    if payload.id == target_id {
        return;
    }

    let painter = response.ctx.layer_painter(response.layer_id);
    if placement == DropPlacement::Parent {
        let color = response.ctx.style().visuals.selection.bg_fill;
        painter.rect_filled(response.rect, 2.0, color.gamma_multiply(0.45));
    } else if sibling_order.contains(&payload.id) {
        paint_sibling_insert_indicator(response, &painter, placement);
    }
}

fn paint_sibling_insert_indicator(
    response: &Response,
    painter: &egui::Painter,
    placement: DropPlacement,
) {
    let y = match placement {
        DropPlacement::Before => response.rect.top(),
        DropPlacement::After => response.rect.bottom(),
        DropPlacement::Parent => return,
    };
    let color = response.ctx.style().visuals.selection.stroke.color;
    painter.line_segment(
        [
            egui::pos2(response.rect.left(), y),
            egui::pos2(response.rect.right(), y),
        ],
        egui::Stroke::new(2.0, color),
    );
}

fn insertion_index(
    target_index: usize,
    sibling_order: &[EntityId],
    payload: SiblingDragPayload,
    placement: DropPlacement,
) -> Option<usize> {
    let sibling_count = sibling_order.len();
    if sibling_count == 0 || target_index >= sibling_count {
        return None;
    }

    let source_index = sibling_order.iter().position(|id| *id == payload.id)?;
    if placement == DropPlacement::Parent {
        return None;
    }
    let insertion_slot = target_index + usize::from(placement == DropPlacement::After);
    let final_index = if source_index < insertion_slot {
        insertion_slot.checked_sub(1)?
    } else {
        insertion_slot
    };

    (final_index < sibling_count).then_some(final_index)
}
