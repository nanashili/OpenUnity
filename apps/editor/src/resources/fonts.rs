//! Reviewed font validation, profile installation and live face identity.

use std::sync::{Arc, OnceLock};

use eframe::egui::{self, Context, FontData, FontDefinitions, FontFamily, Id};

use crate::appearance::AppearanceProfile;

const INTER: &[u8] = local_reference_bytes!("../../assets/unity-7000.0.0a8/Inter-Regular.ttf");
const ROBOTO_MONO: &[u8] =
    local_reference_bytes!("../../assets/unity-7000.0.0a8/RobotoMono-Regular.ttf");
const FONTS: [(&str, &[u8], usize); 2] = [
    ("Inter-Regular", INTER, 431_476),
    ("RobotoMono-Regular", ROBOTO_MONO, 86_908),
];

fn reference_definitions() -> &'static Result<FontDefinitions, String> {
    static DEFINITIONS: OnceLock<Result<FontDefinitions, String>> = OnceLock::new();
    DEFINITIONS.get_or_init(|| {
        if !cfg!(feature = "local-reference-assets") {
            return Err("Local reference fonts unavailable; using egui defaults. Enable local-reference-assets only with the private resource files present.".to_owned());
        }
        let mut definitions = FontDefinitions::default();
        for (name, bytes, expected_bytes) in FONTS {
            if bytes.len() != expected_bytes || bytes.len() > 1_048_576 {
                return Err(format!(
                    "Embedded font {name} failed its reviewed size bound."
                ));
            }
            let data = Arc::new(FontData::from_static(bytes));
            let mut probe = FontDefinitions::empty();
            probe.font_data.insert(name.to_owned(), data.clone());
            // egui's pinned parser panics for unsupported font data. Validate each
            // trusted embedded face once before installing it in a live Context.
            if std::panic::catch_unwind(|| {
                egui::epaint::text::Fonts::new(1024, Default::default(), probe)
            })
            .is_err()
            {
                return Err(format!(
                    "Embedded font {name} could not be parsed; using egui defaults."
                ));
            }
            definitions.font_data.insert(name.to_owned(), data);
        }
        for (family, name) in [
            (FontFamily::Proportional, "Inter-Regular"),
            (FontFamily::Monospace, "RobotoMono-Regular"),
        ] {
            definitions
                .families
                .entry(family)
                .or_default()
                .insert(0, name.to_owned());
        }
        Ok(definitions)
    })
}

pub(crate) fn install(ctx: &Context, profile: AppearanceProfile) -> Option<String> {
    let (definitions, error) = match profile {
        AppearanceProfile::Legacy => (FontDefinitions::default(), None),
        AppearanceProfile::Reference => match reference_definitions() {
            Ok(definitions) => {
                let mut definitions = definitions.clone();
                crate::font_styles::extend_definitions(ctx, &mut definitions);
                (definitions, None)
            }
            Err(error) => (FontDefinitions::default(), Some(error.clone())),
        },
    };
    ctx.data_mut(|d| d.insert_temp(Id::new("editor.font_error"), error.clone()));
    if ctx.fonts(|fonts| fonts.definitions() != &definitions) {
        ctx.set_fonts(definitions);
        // set_fonts takes effect at the next pass. Discard this pass so retained
        // rendering uses the selected definitions, including after Legacy reset.
        ctx.request_discard("Editor font profile changed");
    }
    error
}

fn asset_id(name: &str, data: &FontData) -> Option<&'static str> {
    if !cfg!(feature = "local-reference-assets") {
        return None;
    }
    FONTS.iter().find_map(|(asset, bytes, _)| {
        (name == *asset && data.index == 0 && data.font.as_ref() == *bytes).then_some(*asset)
    })
}

fn asset_hash(asset: Option<&str>) -> Option<&'static str> {
    match asset {
        Some("Inter-Regular") => {
            Some("fc87daef80ebd62ca64506a7bcb999172fcb57f2ab3b022899da2f23fe3cb46c")
        }
        Some("RobotoMono-Regular") => {
            Some("a12917d2df0631d68bfb3217048dbfab217fc9421cdaa8154b86bea0fbd87376")
        }
        _ => None,
    }
}

pub(super) fn families(ctx: &Context) -> Vec<serde_json::Value> {
    ctx.fonts(|fonts| {
        let definitions = fonts.definitions();
        definitions
            .families
            .iter()
            .map(|(family, names)| {
                let entries = names
                    .iter()
                    .filter_map(|name| {
                        definitions.font_data.get(name).map(|data| {
                            serde_json::json!({
                                "name": name,
                                "face_index": data.index,
                                "bytes": data.font.len(),
                                "asset_id": asset_id(name, data),
                                "asset_sha256": asset_hash(asset_id(name, data))
                            })
                        })
                    })
                    .collect::<Vec<_>>();
                let selected = names.first().and_then(|name| {
                    definitions
                        .font_data
                        .get(name)
                        .and_then(|data| asset_id(name, data))
                });
                serde_json::json!({
                    "family": format!("{family:?}"),
                    "fonts": entries,
                    "selected_asset_id": selected
                })
            })
            .collect::<Vec<_>>()
    })
}
