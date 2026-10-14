//! Shared root-row drag behavior for authored hierarchy presentations.

use eframe::egui::{self, Response};
use openunity_editor_core::scene::EntityId;

#[derive(Clone, Copy)]
struct RootDragPayload {
    id: EntityId,
}

pub(crate) struct RootDrop {
    pub operation: Option<(EntityId, usize)>,
    pub completed: bool,
}

pub(crate) fn observe_root_row(
    response: &Response,
    id: EntityId,
    root_position: Option<(usize, usize)>,
    root_order: &[EntityId],
    enabled: bool,
) -> RootDrop {
    let Some((sibling_index, _)) = root_position.filter(|_| enabled) else {
        return RootDrop {
            operation: None,
            completed: false,
        };
    };

    response.dnd_set_drag_payload(RootDragPayload { id });
    paint_drop_indicator(response, id);

    let payload = response.dnd_release_payload::<RootDragPayload>();
    let Some(payload) = payload else {
        return RootDrop {
            operation: None,
            completed: false,
        };
    };

    let sibling_index = insertion_index(response, id, sibling_index, root_order, *payload);
    RootDrop {
        operation: sibling_index.map(|index| (payload.id, index)),
        completed: true,
    }
}

fn paint_drop_indicator(response: &Response, target_id: EntityId) {
    let Some(payload) = response.dnd_hover_payload::<RootDragPayload>() else {
        return;
    };
    if payload.id == target_id {
        return;
    }

    let Some(pointer) = response.interact_pointer_pos() else {
        return;
    };
    let insert_after = pointer.y >= response.rect.center().y;
    let y = if insert_after {
        response.rect.bottom()
    } else {
        response.rect.top()
    };
    let color = response.ctx.style().visuals.selection.stroke.color;
    response.ctx.layer_painter(response.layer_id).line_segment(
        [
            egui::pos2(response.rect.left(), y),
            egui::pos2(response.rect.right(), y),
        ],
        egui::Stroke::new(2.0, color),
    );
}

fn insertion_index(
    response: &Response,
    target_id: EntityId,
    target_index: usize,
    root_order: &[EntityId],
    payload: RootDragPayload,
) -> Option<usize> {
    let sibling_count = root_order.len();
    if payload.id == target_id || sibling_count == 0 || target_index >= sibling_count {
        return None;
    }

    let source_index = root_order.iter().position(|id| *id == payload.id)?;
    let pointer = response.interact_pointer_pos()?;
    let insertion_slot = target_index + usize::from(pointer.y >= response.rect.center().y);
    let final_index = if source_index < insertion_slot {
        insertion_slot.checked_sub(1)?
    } else {
        insertion_slot
    };

    (final_index < sibling_count).then_some(final_index)
}
