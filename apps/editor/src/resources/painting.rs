//! Actual icon painting, successful paint receipts and bounded unavailability.

use std::sync::Arc;

use eframe::egui::{self, Context, Id};

use super::cache::IconCache;
use super::variants::{counted_icon_variant, icon_variant};

// Missing resources are genuine rendering failures, retained once per common role.
fn common_icon_unavailable(ui: &egui::Ui, role: &str) {
    if !matches!(
        role,
        "project_create"
            | "console_info"
            | "console_warning"
            | "console_error"
            | "search_cancel"
            | "search_cancel_active"
            | "search_cancel_off"
    ) {
        return;
    }
    let message =
        format!("{role}: reviewed image unavailable; original semantic control remains in use.");
    ui.ctx().data_mut(|d| {
        let errors = d.get_temp_mut_or_default::<Vec<String>>(Id::new("editor.icon.common_errors"));
        if errors.len() < 7 && !errors.contains(&message) {
            errors.push(message);
        }
    });
}

/// Record successful actual paint calls, rather than requested resource roles.
pub(crate) fn paint(
    ui: &egui::Ui,
    role: &str,
    widget_role: &str,
    widget_id: Id,
    rect: egui::Rect,
) -> bool {
    paint_resolved(ui, role, widget_role, widget_id, rect, None)
}

/// Paint a severity resource resolved from retained records before filtering.
pub(crate) fn paint_counted(
    ui: &egui::Ui,
    role: &str,
    widget_role: &str,
    widget_id: Id,
    rect: egui::Rect,
    retained_count: usize,
) -> bool {
    paint_resolved(ui, role, widget_role, widget_id, rect, Some(retained_count))
}

fn paint_resolved(
    ui: &egui::Ui,
    role: &str,
    widget_role: &str,
    widget_id: Id,
    rect: egui::Rect,
    retained_count: Option<usize>,
) -> bool {
    if !crate::appearance::is_reference(ui) || !ui.is_rect_visible(rect) {
        return false;
    }
    let high_density = ui.ctx().pixels_per_point() > 1.0;
    let asset_id = match retained_count {
        Some(count) => counted_icon_variant(role, high_density, count != 0),
        None => icon_variant(role, high_density),
    };
    let Some(asset_id) = asset_id else {
        common_icon_unavailable(ui, role);
        return false;
    };
    let Some(cache) = ui
        .ctx()
        .data(|d| d.get_temp::<Arc<IconCache>>(Id::new("editor.icon.cache")))
    else {
        common_icon_unavailable(ui, role);
        return false;
    };
    let Some(icon) = cache.icons.get(asset_id) else {
        common_icon_unavailable(ui, role);
        return false;
    };
    ui.painter().image(
        icon.texture.id(),
        rect,
        egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1.0, 1.0)),
        egui::Color32::WHITE,
    );
    let mut record = serde_json::json!({
        "role": role,
        "widget_role": widget_role,
        "widget_id": widget_id.value(),
        "texture_id": format!("{:?}", icon.texture.id()),
        "asset_id": asset_id,
        "source_dimensions": icon.dimensions,
        "logical_rect": crate::appearance::rect_json(rect),
        "pixels_per_point": ui.ctx().pixels_per_point(),
        "variant_scale": if high_density { 2 } else { 1 }
    });
    add_presentation_state(&mut record, role, retained_count);
    retain_record(ui.ctx(), record);
    true
}

fn add_presentation_state(
    record: &mut serde_json::Value,
    role: &str,
    retained_count: Option<usize>,
) {
    if let Some(count) = retained_count {
        record["presentation_state"] = if count == 0 {
            "zero_count"
        } else {
            "positive_count"
        }
        .into();
        record["retained_severity_count"] = count.into();
    } else if role == "project_create" {
        record["presentation_state"] = "ordinary".into();
    } else if let Some(state) = match role {
        "search_cancel" => Some("normal"),
        "search_cancel_active" => Some("active"),
        "search_cancel_off" => Some("off"),
        _ => None,
    } {
        record["presentation_state"] = state.into();
    }
}

fn retain_record(ctx: &Context, record: serde_json::Value) {
    ctx.data_mut(|d| {
        let records = d.get_temp_mut_or_default::<Vec<serde_json::Value>>(Id::new(
            "editor.icon.paint_records",
        ));
        if records.len() < 4096 {
            records.push(record);
        } else {
            d.insert_temp(Id::new("editor.icon.records_truncated"), true);
        }
    });
}
