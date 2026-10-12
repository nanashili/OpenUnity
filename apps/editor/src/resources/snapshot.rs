//! Live font, cached texture, failure and successful-paint observations.

use std::sync::Arc;

use eframe::egui::{Context, Id};

use super::cache::IconCache;

struct IconSnapshot {
    cached_icons: Vec<serde_json::Value>,
    load_errors: Vec<String>,
    paint_records: Vec<serde_json::Value>,
    truncated: bool,
}

/// Read the live font definitions, never a requested or cached inventory.
pub(crate) fn snapshot(ctx: &Context) -> serde_json::Value {
    let families = super::fonts::families(ctx);
    let error = ctx
        .data(|d| d.get_temp::<Option<String>>(Id::new("editor.font_error")))
        .flatten();
    let icons = icon_snapshot(ctx);
    serde_json::json!({
        "font_families": families,
        "load_error": error,
        "local_reference_assets_enabled": cfg!(feature = "local-reference-assets"),
        "manifest_sha256": cfg!(feature = "local-reference-assets")
            .then_some("9ad381ac0cf1e644b86a739676142a3873b8a233d91e8aa484eff04cfd38215e"),
        "identity_check": "Approved IDs and manifest hashes are reported only after exact embedded byte and face-index comparison; acceptance independently hashes active bytes.",
        "fallback_note": "The ordered egui fallback faces remain available for missing glyphs; exact per-glyph Unity fallback and platform rasterization are unverified.",
        "paint_records": icons.paint_records,
        "cached_icons": icons.cached_icons,
        "icon_load_errors": icons.load_errors,
        "paint_records_truncated": icons.truncated,
        "icon_assets": if cfg!(feature = "local-reference-assets") {
            "55 reviewed variants for28 explicit roles; runtime Unity role equivalence remains unverified"
        } else {
            "Local reference icons unavailable; existing semantic glyph and geometry fallbacks remain active"
        }
    })
}

fn icon_snapshot(ctx: &Context) -> IconSnapshot {
    let cache = ctx.data(|d| d.get_temp::<Arc<IconCache>>(Id::new("editor.icon.cache")));
    let cached_icons = cache
        .as_ref()
        .map(|cache| {
            cache
                .icons
                .iter()
                .map(|(asset_id, icon)| {
                    serde_json::json!({
                        "asset_id": asset_id,
                        "texture_id": format!("{:?}", icon.texture.id()),
                        "source_dimensions": icon.dimensions
                    })
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let mut icon_load_errors = cache
        .as_ref()
        .map(|cache| cache.errors.clone())
        .unwrap_or_default();
    icon_load_errors.extend(
        ctx.data(|d| d.get_temp::<Vec<String>>(Id::new("editor.icon.common_errors")))
            .unwrap_or_default(),
    );
    let paint_records = ctx
        .data(|d| d.get_temp::<Vec<serde_json::Value>>(Id::new("editor.icon.paint_records")))
        .unwrap_or_default();
    let truncated = ctx
        .data(|d| d.get_temp::<bool>(Id::new("editor.icon.records_truncated")))
        .unwrap_or(false);
    IconSnapshot {
        cached_icons,
        load_errors: icon_load_errors,
        paint_records,
        truncated,
    }
}
