use std::collections::BTreeSet;

use eframe::egui::Ui;
use openunity_editor_core::scene::{Entity, EntityId};

use crate::EditorAction;

pub(super) struct EntityRow<'a> {
    pub entity: &'a Entity,
    pub depth: usize,
    pub sibling_position: Option<(usize, usize)>,
    pub sibling_order: &'a [EntityId],
    pub has_children: bool,
    pub collapsed: bool,
}

pub(super) fn render_entity_row(
    ui: &mut Ui,
    row: EntityRow<'_>,
    selected: &BTreeSet<EntityId>,
    actions: &mut Vec<EditorAction>,
) -> Option<bool> {
    let EntityRow {
        entity,
        depth,
        sibling_position,
        sibling_order,
        has_children,
        collapsed,
    } = row;
    let row_response = ui.push_id(entity.id.0, |ui| {
        ui.horizontal(|ui| {
            ui.add_space(depth as f32 * 14.0);
            let disclosure_toggle = disclosure_control(ui, has_children, collapsed);
            let label = format!(
                "{} ({})",
                entity.name,
                if entity.active { "active" } else { "inactive" }
            );
            let controls_width = if depth > 0
                && sibling_position.is_some_and(|(_, sibling_count)| sibling_count > 1)
            {
                44.0
            } else {
                0.0
            };
            let row_width = (ui.available_width() - controls_width).max(0.0);
            let response = ui.add(
                eframe::egui::Button::selectable(selected.contains(&entity.id), label)
                    .sense(eframe::egui::Sense::click_and_drag())
                    .min_size(eframe::egui::vec2(row_width, 18.0))
                    .frame(false),
            );
            if depth > 0 {
                sibling_controls(ui, entity.id, sibling_position, actions);
            }
            (response, disclosure_toggle)
        })
    });
    let (response, disclosure_toggle) = row_response.inner.inner;
    if disclosure_toggle.is_some() {
        return disclosure_toggle;
    }

    let drop = super::super::super::sibling_drag::observe_sibling_row(
        &response,
        entity.id,
        sibling_position,
        sibling_order,
        true,
    );
    if let Some(operation) = drop.operation {
        match operation {
            super::super::super::sibling_drag::SiblingDropOperation::Reorder {
                id,
                sibling_index,
            } => actions.push(EditorAction::SetSiblingIndex { id, sibling_index }),
            super::super::super::sibling_drag::SiblingDropOperation::Reparent { id, parent } => {
                actions.push(EditorAction::Reparent {
                    id,
                    parent: Some(parent),
                });
            }
        }
        return None;
    }
    if drop.completed {
        return None;
    }

    if response.clicked() {
        let toggle = ui.input(|input| input.modifiers.command || input.modifiers.ctrl);
        actions.push(EditorAction::Select {
            id: entity.id,
            toggle,
        });
    }

    None
}

fn disclosure_control(ui: &mut Ui, has_children: bool, collapsed: bool) -> Option<bool> {
    if !has_children {
        ui.add_space(18.0);
        return None;
    }

    let glyph = if collapsed { "▸" } else { "▾" };
    let hover_text = if collapsed {
        "Expand branch"
    } else {
        "Collapse branch"
    };
    let response = ui.small_button(glyph).on_hover_text(hover_text);

    response
        .clicked()
        .then(|| ui.input(|input| input.modifiers.alt))
}

fn sibling_controls(
    ui: &mut Ui,
    id: EntityId,
    sibling_position: Option<(usize, usize)>,
    actions: &mut Vec<EditorAction>,
) {
    let Some((sibling_index, sibling_count)) = sibling_position else {
        return;
    };
    if sibling_count < 2 {
        return;
    }

    if sibling_index > 0
        && ui
            .small_button("↟")
            .on_hover_text("Set as first sibling")
            .clicked()
    {
        actions.push(EditorAction::SetSiblingIndex {
            id,
            sibling_index: 0,
        });
    }
    if sibling_index + 1 < sibling_count
        && ui
            .small_button("↡")
            .on_hover_text("Set as last sibling")
            .clicked()
    {
        actions.push(EditorAction::SetSiblingIndex {
            id,
            sibling_index: sibling_count - 1,
        });
    }
}
