//! Register explicit style families while preserving existing fallback order.

use std::collections::BTreeMap;
use std::sync::Arc;

use eframe::egui::{Context, FontData, FontDefinitions, FontFamily, Id};

use super::ProportionalStyleV1;

pub(super) type RegisteredFaces = BTreeMap<String, Arc<FontData>>;

// Called by the existing profile lifecycle before layout. Named families do not
// alter the proportional/monospace defaults or their ordered fallback chains.
pub(crate) fn extend_definitions(ctx: &Context, definitions: &mut FontDefinitions) {
    let registered = ctx
        .data(|d| d.get_temp::<RegisteredFaces>(Id::new("editor.font_styles.v1.registered")))
        .unwrap_or_default();
    let fallback = definitions
        .families
        .get(&FontFamily::Proportional)
        .cloned()
        .unwrap_or_default();
    for (name, data) in registered {
        let style = match name.as_str() {
            "Inter-SemiBold" => ProportionalStyleV1::Semibold,
            "Inter-Italic" => ProportionalStyleV1::Italic,
            "Inter-SemiBoldItalic" => ProportionalStyleV1::SemiboldItalic,
            _ => continue,
        };
        definitions.font_data.insert(name.clone(), data);
        let mut names = vec![name];
        names.extend(fallback.clone());
        definitions.families.insert(style.family(), names);
    }
}
