//! Reviewed embedded fonts for INFRA-EDITOR-VISUAL-ASSETS-001.

use std::sync::{Arc, OnceLock};

use eframe::egui::{self, Context, FontData, FontDefinitions, FontFamily, Id};

use crate::appearance::AppearanceProfile;

const INTER: &[u8] = local_reference_bytes!("../assets/unity-7000.0.0a8/Inter-Regular.ttf");
const ROBOTO_MONO: &[u8] =
    local_reference_bytes!("../assets/unity-7000.0.0a8/RobotoMono-Regular.ttf");
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

/// Read the live font definitions, never a requested or cached inventory.
pub(crate) fn snapshot(ctx: &Context) -> serde_json::Value {
    let families = ctx.fonts(|fonts| {
        let definitions = fonts.definitions();
        definitions.families.iter().map(|(family, names)| {
            let entries = names.iter().filter_map(|name| {
                definitions.font_data.get(name).map(|data| serde_json::json!({
                    "name":name,"face_index":data.index,"bytes":data.font.len(),"asset_id":asset_id(name,data),
                    "asset_sha256":asset_hash(asset_id(name,data))
                }))
            }).collect::<Vec<_>>();
            let selected = names.first().and_then(|name| definitions.font_data.get(name).and_then(|data| asset_id(name,data)));
            serde_json::json!({"family":format!("{family:?}"),"fonts":entries,"selected_asset_id":selected})
        }).collect::<Vec<_>>()
    });
    let error = ctx
        .data(|d| d.get_temp::<Option<String>>(Id::new("editor.font_error")))
        .flatten();
    let cache = ctx.data(|d| d.get_temp::<Arc<IconCache>>(Id::new("editor.icon.cache")));
    let cached_icons = cache.as_ref().map(|cache| cache.icons.iter().map(|(asset_id,icon)|
        serde_json::json!({"asset_id":asset_id,"texture_id":format!("{:?}",icon.texture.id()),"source_dimensions":icon.dimensions})).collect::<Vec<_>>()).unwrap_or_default();
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
    serde_json::json!({"font_families":families,"load_error":error,
    "local_reference_assets_enabled":cfg!(feature = "local-reference-assets"),
    "manifest_sha256":cfg!(feature = "local-reference-assets").then_some("9ad381ac0cf1e644b86a739676142a3873b8a233d91e8aa484eff04cfd38215e"),
    "identity_check":"Approved IDs and manifest hashes are reported only after exact embedded byte and face-index comparison; acceptance independently hashes active bytes.",
    "fallback_note":"The ordered egui fallback faces remain available for missing glyphs; exact per-glyph Unity fallback and platform rasterization are unverified.",
    "paint_records":paint_records,"cached_icons":cached_icons,"icon_load_errors":icon_load_errors,
    "paint_records_truncated":truncated,"icon_assets":if cfg!(feature = "local-reference-assets") {
        "55 reviewed variants for28 explicit roles; runtime Unity role equivalence remains unverified"
    } else {
        "Local reference icons unavailable; existing semantic glyph and geometry fallbacks remain active"
    }})
}

// The31 original, four Inspector lock,14 common-control and six search-cancel variants are embedded.
// Unselected provenance inventory never enters the cache.
struct IconAsset {
    id: &'static str,
    bytes: &'static [u8],
    dimensions: [u32; 2],
    expected_bytes: usize,
}
const ICONS: &[IconAsset] = &[
    IconAsset {
        id: "unity-icon:6368833644570539117",
        bytes: local_reference_bytes!(
            "../assets/unity-7000.0.0a8/icons/in-lockbutton-6368833644570539117.png"
        ),
        dimensions: [16, 16],
        expected_bytes: 327,
    },
    IconAsset {
        id: "unity-icon:-2756979610131851051",
        bytes: local_reference_bytes!(
            "../assets/unity-7000.0.0a8/icons/in-lockbutton-on--2756979610131851051.png"
        ),
        dimensions: [16, 16],
        expected_bytes: 284,
    },
    IconAsset {
        id: "unity-icon:8554052077915636359",
        bytes: local_reference_bytes!(
            "../assets/unity-7000.0.0a8/icons/in-lockbutton-2x-8554052077915636359.png"
        ),
        dimensions: [32, 32],
        expected_bytes: 356,
    },
    IconAsset {
        id: "unity-icon:3623881695602462711",
        bytes: local_reference_bytes!(
            "../assets/unity-7000.0.0a8/icons/in-lockbutton-on-2x-3623881695602462711.png"
        ),
        dimensions: [32, 32],
        expected_bytes: 320,
    },
    IconAsset {
        id: "unity-icon:-8204150473280056172",
        bytes: local_reference_bytes!(
            "../assets/unity-7000.0.0a8/icons/d-cs-script-icon--8204150473280056172.png"
        ),
        dimensions: [256, 256],
        expected_bytes: 4396,
    },
    IconAsset {
        id: "unity-icon:-8183847732512111428",
        bytes: local_reference_bytes!(
            "../assets/unity-7000.0.0a8/icons/d-in-foldout-on--8183847732512111428.png"
        ),
        dimensions: [13, 13],
        expected_bytes: 212,
    },
    IconAsset {
        id: "unity-icon:-7606759526039521141",
        bytes: local_reference_bytes!(
            "../assets/unity-7000.0.0a8/icons/d-gameobject-icon--7606759526039521141.png"
        ),
        dimensions: [256, 256],
        expected_bytes: 8184,
    },
    IconAsset {
        id: "unity-icon:-7311690078608690865",
        bytes: local_reference_bytes!(
            "../assets/unity-7000.0.0a8/icons/d-in-foldout-on-2x--7311690078608690865.png"
        ),
        dimensions: [26, 26],
        expected_bytes: 355,
    },
    IconAsset {
        id: "unity-icon:-6423792434712278376",
        bytes: local_reference_bytes!(
            "../assets/unity-7000.0.0a8/icons/d-unityeditor-gameview--6423792434712278376.png"
        ),
        dimensions: [16, 16],
        expected_bytes: 261,
    },
    IconAsset {
        id: "unity-icon:-6182031959114119803",
        bytes: local_reference_bytes!(
            "../assets/unity-7000.0.0a8/icons/d-search-icon-2x--6182031959114119803.png"
        ),
        dimensions: [16, 16],
        expected_bytes: 345,
    },
    IconAsset {
        id: "unity-icon:-5467254957812901981",
        bytes: local_reference_bytes!(
            "../assets/unity-7000.0.0a8/icons/d-project--5467254957812901981.png"
        ),
        dimensions: [16, 16],
        expected_bytes: 164,
    },
    IconAsset {
        id: "unity-icon:-5179483145760003458",
        bytes: local_reference_bytes!(
            "../assets/unity-7000.0.0a8/icons/d-project-2x--5179483145760003458.png"
        ),
        dimensions: [32, 32],
        expected_bytes: 225,
    },
    IconAsset {
        id: "unity-icon:-4950941429401207979",
        bytes: local_reference_bytes!(
            "../assets/unity-7000.0.0a8/icons/d-unityeditor-consolewindow-2x--4950941429401207979.png"
        ),
        dimensions: [32, 32],
        expected_bytes: 219,
    },
    IconAsset {
        id: "unity-icon:-4819823979207714598",
        bytes: local_reference_bytes!(
            "../assets/unity-7000.0.0a8/icons/d-playbutton-2x--4819823979207714598.png"
        ),
        dimensions: [32, 32],
        expected_bytes: 332,
    },
    IconAsset {
        id: "unity-icon:-4327648978806127646",
        bytes: local_reference_bytes!(
            "../assets/unity-7000.0.0a8/icons/d-unityeditor-consolewindow--4327648978806127646.png"
        ),
        dimensions: [16, 16],
        expected_bytes: 153,
    },
    IconAsset {
        id: "unity-icon:-3883791630232141078",
        bytes: local_reference_bytes!(
            "../assets/unity-7000.0.0a8/icons/d-in-foldout-2x--3883791630232141078.png"
        ),
        dimensions: [26, 26],
        expected_bytes: 378,
    },
    IconAsset {
        id: "unity-icon:-3734745235275155857",
        bytes: local_reference_bytes!(
            "../assets/unity-7000.0.0a8/icons/d-unityeditor-scenehierarchywindow-2x--3734745235275155857.png"
        ),
        dimensions: [32, 32],
        expected_bytes: 193,
    },
    IconAsset {
        id: "unity-icon:-3290981458908399999",
        bytes: local_reference_bytes!(
            "../assets/unity-7000.0.0a8/icons/d-search-icon--3290981458908399999.png"
        ),
        dimensions: [8, 8],
        expected_bytes: 198,
    },
    IconAsset {
        id: "unity-icon:-2667387946076563598",
        bytes: local_reference_bytes!(
            "../assets/unity-7000.0.0a8/icons/d-unityeditor-inspectorwindow--2667387946076563598.png"
        ),
        dimensions: [16, 16],
        expected_bytes: 219,
    },
    IconAsset {
        id: "unity-icon:-2633161165857034927",
        bytes: local_reference_bytes!(
            "../assets/unity-7000.0.0a8/icons/d-in-foldout--2633161165857034927.png"
        ),
        dimensions: [13, 13],
        expected_bytes: 234,
    },
    IconAsset {
        id: "unity-icon:-440750813802333266",
        bytes: local_reference_bytes!(
            "../assets/unity-7000.0.0a8/icons/d-unityeditor-inspectorwindow-2x--440750813802333266.png"
        ),
        dimensions: [32, 32],
        expected_bytes: 420,
    },
    IconAsset {
        id: "unity-icon:-157328457287382343",
        bytes: local_reference_bytes!(
            "../assets/unity-7000.0.0a8/icons/d-pausebutton--157328457287382343.png"
        ),
        dimensions: [16, 16],
        expected_bytes: 126,
    },
    IconAsset {
        id: "unity-icon:2593428753322112591",
        bytes: local_reference_bytes!(
            "../assets/unity-7000.0.0a8/icons/d-unityeditor-sceneview-2593428753322112591.png"
        ),
        dimensions: [16, 16],
        expected_bytes: 141,
    },
    IconAsset {
        id: "unity-icon:3764734208316478041",
        bytes: local_reference_bytes!(
            "../assets/unity-7000.0.0a8/icons/d-folder-icon-3764734208316478041.png"
        ),
        dimensions: [256, 256],
        expected_bytes: 1532,
    },
    IconAsset {
        id: "unity-icon:4387052765368226613",
        bytes: local_reference_bytes!(
            "../assets/unity-7000.0.0a8/icons/d-folderempty-icon-4387052765368226613.png"
        ),
        dimensions: [256, 256],
        expected_bytes: 1741,
    },
    IconAsset {
        id: "unity-icon:4621777727084837110",
        bytes: local_reference_bytes!(
            "../assets/unity-7000.0.0a8/icons/d-unityeditor-gameview-2x-4621777727084837110.png"
        ),
        dimensions: [32, 32],
        expected_bytes: 405,
    },
    IconAsset {
        id: "unity-icon:5104857897416931454",
        bytes: local_reference_bytes!(
            "../assets/unity-7000.0.0a8/icons/d-pausebutton-2x-5104857897416931454.png"
        ),
        dimensions: [32, 32],
        expected_bytes: 208,
    },
    IconAsset {
        id: "unity-icon:6115457227726860241",
        bytes: local_reference_bytes!(
            "../assets/unity-7000.0.0a8/icons/d-folderopened-icon-6115457227726860241.png"
        ),
        dimensions: [256, 256],
        expected_bytes: 2739,
    },
    IconAsset {
        id: "unity-icon:7481816853378517098",
        bytes: local_reference_bytes!(
            "../assets/unity-7000.0.0a8/icons/d-stepbutton-2x-7481816853378517098.png"
        ),
        dimensions: [32, 32],
        expected_bytes: 359,
    },
    IconAsset {
        id: "unity-icon:7606677459205826508",
        bytes: local_reference_bytes!(
            "../assets/unity-7000.0.0a8/icons/d-stepbutton-7606677459205826508.png"
        ),
        dimensions: [16, 16],
        expected_bytes: 220,
    },
    IconAsset {
        id: "unity-icon:7966133145522015247",
        bytes: local_reference_bytes!(
            "../assets/unity-7000.0.0a8/icons/d-unityeditor-scenehierarchywindow-7966133145522015247.png"
        ),
        dimensions: [16, 16],
        expected_bytes: 162,
    },
    IconAsset {
        id: "unity-icon:8119404038100594556",
        bytes: local_reference_bytes!(
            "../assets/unity-7000.0.0a8/icons/d-playbutton-8119404038100594556.png"
        ),
        dimensions: [16, 16],
        expected_bytes: 214,
    },
    IconAsset {
        id: "unity-icon:8316842575475577922",
        bytes: local_reference_bytes!(
            "../assets/unity-7000.0.0a8/icons/d-textasset-icon-8316842575475577922.png"
        ),
        dimensions: [256, 256],
        expected_bytes: 2657,
    },
    IconAsset {
        id: "unity-icon:8634526014445323508",
        bytes: local_reference_bytes!(
            "../assets/unity-7000.0.0a8/icons/d-unityeditor-sceneview-2x-8634526014445323508.png"
        ),
        dimensions: [32, 32],
        expected_bytes: 194,
    },
    IconAsset {
        id: "unity-icon:8779834242249542326",
        bytes: local_reference_bytes!(
            "../assets/unity-7000.0.0a8/icons/d-defaultasset-icon-8779834242249542326.png"
        ),
        dimensions: [64, 64],
        expected_bytes: 382,
    },
    IconAsset {
        id: "unity-icon:-7670031800153505217",
        bytes: local_reference_bytes!(
            "../assets/unity-7000.0.0a8/common-icons/d-console-infoicon-inactive-sml-2x---7670031800153505217.png"
        ),
        dimensions: [32, 32],
        expected_bytes: 640,
    },
    IconAsset {
        id: "unity-icon:-6520636669904382973",
        bytes: local_reference_bytes!(
            "../assets/unity-7000.0.0a8/common-icons/d-console-warnicon-inactive-sml-2x---6520636669904382973.png"
        ),
        dimensions: [32, 32],
        expected_bytes: 454,
    },
    IconAsset {
        id: "unity-icon:-6442799736734524024",
        bytes: local_reference_bytes!(
            "../assets/unity-7000.0.0a8/common-icons/d-console-erroricon-sml-2x---6442799736734524024.png"
        ),
        dimensions: [32, 32],
        expected_bytes: 446,
    },
    IconAsset {
        id: "unity-icon:-4405542200666886037",
        bytes: local_reference_bytes!(
            "../assets/unity-7000.0.0a8/common-icons/d-console-infoicon-sml-2x---4405542200666886037.png"
        ),
        dimensions: [32, 32],
        expected_bytes: 685,
    },
    IconAsset {
        id: "unity-icon:-4166112459083603383",
        bytes: local_reference_bytes!(
            "../assets/unity-7000.0.0a8/common-icons/d-console-warnicon-sml---4166112459083603383.png"
        ),
        dimensions: [16, 16],
        expected_bytes: 341,
    },
    IconAsset {
        id: "unity-icon:-3276413875869322032",
        bytes: local_reference_bytes!(
            "../assets/unity-7000.0.0a8/common-icons/d-console-infoicon-inactive-sml---3276413875869322032.png"
        ),
        dimensions: [16, 16],
        expected_bytes: 329,
    },
    IconAsset {
        id: "unity-icon:-2213165035764607753",
        bytes: local_reference_bytes!(
            "../assets/unity-7000.0.0a8/common-icons/d-toolbar-plus-more---2213165035764607753.png"
        ),
        dimensions: [16, 16],
        expected_bytes: 178,
    },
    IconAsset {
        id: "unity-icon:-1683012324619503922",
        bytes: local_reference_bytes!(
            "../assets/unity-7000.0.0a8/common-icons/d-console-erroricon-inactive-sml-2x---1683012324619503922.png"
        ),
        dimensions: [32, 32],
        expected_bytes: 436,
    },
    IconAsset {
        id: "unity-icon:-1422735066093442274",
        bytes: local_reference_bytes!(
            "../assets/unity-7000.0.0a8/common-icons/d-console-erroricon-inactive-sml---1422735066093442274.png"
        ),
        dimensions: [16, 16],
        expected_bytes: 265,
    },
    IconAsset {
        id: "unity-icon:786882066299589297",
        bytes: local_reference_bytes!(
            "../assets/unity-7000.0.0a8/common-icons/d-console-infoicon-sml--786882066299589297.png"
        ),
        dimensions: [16, 16],
        expected_bytes: 388,
    },
    IconAsset {
        id: "unity-icon:2554409044035794638",
        bytes: local_reference_bytes!(
            "../assets/unity-7000.0.0a8/common-icons/d-toolbar-plus-more-2x--2554409044035794638.png"
        ),
        dimensions: [32, 32],
        expected_bytes: 317,
    },
    IconAsset {
        id: "unity-icon:8521385900707336378",
        bytes: local_reference_bytes!(
            "../assets/unity-7000.0.0a8/common-icons/d-console-erroricon-sml--8521385900707336378.png"
        ),
        dimensions: [16, 16],
        expected_bytes: 308,
    },
    IconAsset {
        id: "unity-icon:8751020510356332213",
        bytes: local_reference_bytes!(
            "../assets/unity-7000.0.0a8/common-icons/d-console-warnicon-inactive-sml--8751020510356332213.png"
        ),
        dimensions: [16, 16],
        expected_bytes: 262,
    },
    IconAsset {
        id: "unity-icon:8889457268015356735",
        bytes: local_reference_bytes!(
            "../assets/unity-7000.0.0a8/common-icons/d-console-warnicon-sml-2x--8889457268015356735.png"
        ),
        dimensions: [32, 32],
        expected_bytes: 587,
    },
    IconAsset {
        id: "unity-icon:-5230443368201148223",
        bytes: local_reference_bytes!(
            "../assets/unity-7000.0.0a8/search-cancel/toolbarsearchcancelbutton---5230443368201148223.png"
        ),
        dimensions: [14, 14],
        expected_bytes: 454,
    },
    IconAsset {
        id: "unity-icon:3847086183118950969",
        bytes: local_reference_bytes!(
            "../assets/unity-7000.0.0a8/search-cancel/toolbarsearchcancelbutton-2x--3847086183118950969.png"
        ),
        dimensions: [28, 28],
        expected_bytes: 805,
    },
    IconAsset {
        id: "unity-icon:1146234159502602164",
        bytes: local_reference_bytes!(
            "../assets/unity-7000.0.0a8/search-cancel/toolbarsearchcancelbuttonactive--1146234159502602164.png"
        ),
        dimensions: [14, 14],
        expected_bytes: 460,
    },
    IconAsset {
        id: "unity-icon:8100730101251085683",
        bytes: local_reference_bytes!(
            "../assets/unity-7000.0.0a8/search-cancel/toolbarsearchcancelbuttonactive-2x--8100730101251085683.png"
        ),
        dimensions: [28, 28],
        expected_bytes: 815,
    },
    IconAsset {
        id: "unity-icon:4511231667045328251",
        bytes: local_reference_bytes!(
            "../assets/unity-7000.0.0a8/search-cancel/toolbarsearchcancelbuttonoff--4511231667045328251.png"
        ),
        dimensions: [14, 14],
        expected_bytes: 305,
    },
    IconAsset {
        id: "unity-icon:-2936632161100208059",
        bytes: local_reference_bytes!(
            "../assets/unity-7000.0.0a8/search-cancel/toolbarsearchcancelbuttonoff-2x---2936632161100208059.png"
        ),
        dimensions: [28, 28],
        expected_bytes: 622,
    },
];

fn icon_variant(role: &str, high_density: bool) -> Option<&'static str> {
    Some(match (role, high_density) {
        ("search_cancel", false) => "unity-icon:-5230443368201148223",
        ("search_cancel", true) => "unity-icon:3847086183118950969",
        ("search_cancel_active", false) => "unity-icon:1146234159502602164",
        ("search_cancel_active", true) => "unity-icon:8100730101251085683",
        ("search_cancel_off", false) => "unity-icon:4511231667045328251",
        ("search_cancel_off", true) => "unity-icon:-2936632161100208059",
        ("project_create", false) => "unity-icon:-2213165035764607753",
        ("project_create", true) => "unity-icon:2554409044035794638",
        ("lock_unlocked", false) => "unity-icon:6368833644570539117",
        ("lock_unlocked", true) => "unity-icon:8554052077915636359",
        ("lock_locked", false) => "unity-icon:-2756979610131851051",
        ("lock_locked", true) => "unity-icon:3623881695602462711",
        ("transport_play", false) => "unity-icon:8119404038100594556",
        ("transport_play", true) => "unity-icon:-4819823979207714598",
        ("transport_pause", false) => "unity-icon:-157328457287382343",
        ("transport_pause", true) => "unity-icon:5104857897416931454",
        ("transport_step", false) => "unity-icon:7606677459205826508",
        ("transport_step", true) => "unity-icon:7481816853378517098",
        ("tab_hierarchy", false) => "unity-icon:7966133145522015247",
        ("tab_hierarchy", true) => "unity-icon:-3734745235275155857",
        ("tab_scene", false) => "unity-icon:2593428753322112591",
        ("tab_scene", true) => "unity-icon:8634526014445323508",
        ("tab_game", false) => "unity-icon:-6423792434712278376",
        ("tab_game", true) => "unity-icon:4621777727084837110",
        ("tab_project", false) => "unity-icon:-5467254957812901981",
        ("tab_project", true) => "unity-icon:-5179483145760003458",
        ("tab_console", false) => "unity-icon:-4327648978806127646",
        ("tab_console", true) => "unity-icon:-4950941429401207979",
        ("tab_inspector", false) => "unity-icon:-2667387946076563598",
        ("tab_inspector", true) => "unity-icon:-440750813802333266",
        ("search", false) => "unity-icon:-3290981458908399999",
        ("search", true) => "unity-icon:-6182031959114119803",
        ("disclosure_closed", false) => "unity-icon:-2633161165857034927",
        ("disclosure_closed", true) => "unity-icon:-3883791630232141078",
        ("disclosure_open", false) => "unity-icon:-8183847732512111428",
        ("disclosure_open", true) => "unity-icon:-7311690078608690865",
        ("game_object", _) => "unity-icon:-7606759526039521141",
        ("folder", _) => "unity-icon:3764734208316478041",
        ("folder_open", _) => "unity-icon:6115457227726860241",
        ("folder_empty", _) => "unity-icon:4387052765368226613",
        ("file", _) => "unity-icon:8779834242249542326",
        ("text_file", _) => "unity-icon:8316842575475577922",
        ("cs_file", _) => "unity-icon:-8204150473280056172",
        _ => return None,
    })
}

fn counted_icon_variant(role: &str, high_density: bool, positive: bool) -> Option<&'static str> {
    Some(match (role, high_density, positive) {
        ("console_info", false, true) => "unity-icon:786882066299589297",
        ("console_info", true, true) => "unity-icon:-4405542200666886037",
        ("console_info", false, false) => "unity-icon:-3276413875869322032",
        ("console_info", true, false) => "unity-icon:-7670031800153505217",
        ("console_warning", false, true) => "unity-icon:-4166112459083603383",
        ("console_warning", true, true) => "unity-icon:8889457268015356735",
        ("console_warning", false, false) => "unity-icon:8751020510356332213",
        ("console_warning", true, false) => "unity-icon:-6520636669904382973",
        ("console_error", false, true) => "unity-icon:8521385900707336378",
        ("console_error", true, true) => "unity-icon:-6442799736734524024",
        ("console_error", false, false) => "unity-icon:-1422735066093442274",
        ("console_error", true, false) => "unity-icon:-1683012324619503922",
        _ => return None,
    })
}

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

struct CachedIcon {
    texture: egui::TextureHandle,
    dimensions: [u32; 2],
}
#[derive(Default)]
struct IconCache {
    icons: std::collections::BTreeMap<&'static str, CachedIcon>,
    errors: Vec<String>,
}

fn decode_icon(asset: &IconAsset) -> Result<egui::ColorImage, String> {
    if asset.bytes.len() != asset.expected_bytes || asset.bytes.len() > 65_536 {
        return Err("encoded size is outside the reviewed bound".into());
    }
    let decoder = png::Decoder::new_with_limits(
        std::io::Cursor::new(asset.bytes),
        png::Limits { bytes: 1_048_576 },
    );
    let mut reader = decoder.read_info().map_err(|e| e.to_string())?;
    let info = reader.info();
    if [info.width, info.height] != asset.dimensions
        || info.width == 0
        || info.height == 0
        || info.width > 256
        || info.height > 256
        || info.color_type != png::ColorType::Rgba
        || info.bit_depth != png::BitDepth::Eight
    {
        return Err("dimensions or RGBA8 format differ from the reviewed resource".into());
    }
    let size = reader
        .output_buffer_size()
        .filter(|size| *size <= 262_144)
        .ok_or_else(|| "decoded image exceeds the reviewed allocation bound".to_owned())?;
    let expected = asset.dimensions[0] as usize * asset.dimensions[1] as usize * 4;
    if size != expected {
        return Err("decoded RGBA size differs from declared dimensions".into());
    }
    let mut rgba = vec![0; size];
    let output = reader.next_frame(&mut rgba).map_err(|e| e.to_string())?;
    if output.buffer_size() != expected {
        return Err("decoded frame has an unexpected size".into());
    }
    // Approved PNGs already use top-left orientation and unassociated RGBA.
    Ok(egui::ColorImage::from_rgba_unmultiplied(
        [asset.dimensions[0] as usize, asset.dimensions[1] as usize],
        &rgba,
    ))
}

/// Clear receipts every retained UI pass. Each Context owns its textures.
pub(crate) fn begin_frame(ctx: &Context, profile: AppearanceProfile) -> Option<String> {
    ctx.data_mut(|d| {
        d.insert_temp(
            Id::new("editor.icon.paint_records"),
            Vec::<serde_json::Value>::new(),
        );
        d.insert_temp(Id::new("editor.icon.records_truncated"), false);
    });
    if profile == AppearanceProfile::Legacy
        || ctx.data(|d| {
            d.get_temp::<Arc<IconCache>>(Id::new("editor.icon.cache"))
                .is_some()
        })
    {
        return None;
    }
    let mut cache = IconCache::default();
    if cfg!(feature = "local-reference-assets") {
        for asset in ICONS {
            match decode_icon(asset) {
                Ok(image) => {
                    let texture = ctx.load_texture(asset.id, image, egui::TextureOptions::LINEAR);
                    cache.icons.insert(
                        asset.id,
                        CachedIcon {
                            texture,
                            dimensions: asset.dimensions,
                        },
                    );
                }
                Err(error) => cache.errors.push(format!(
                    "{}: {error}; independent geometry remains in use.",
                    asset.id
                )),
            }
        }
    } else {
        cache.errors.push("Local reference icons unavailable; existing semantic glyph and geometry fallbacks remain active.".to_owned());
    }
    let error = (!cache.errors.is_empty()).then(|| cache.errors.join("\n"));
    ctx.data_mut(|d| d.insert_temp(Id::new("editor.icon.cache"), Arc::new(cache)));
    error
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
    let mut record = serde_json::json!({"role":role,"widget_role":widget_role,"widget_id":widget_id.value(),
        "texture_id":format!("{:?}",icon.texture.id()),"asset_id":asset_id,"source_dimensions":icon.dimensions,
        "logical_rect":crate::appearance::rect_json(rect),"pixels_per_point":ui.ctx().pixels_per_point(),
        "variant_scale":if high_density {2} else {1}});
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
    ui.ctx().data_mut(|d| {
        let records = d.get_temp_mut_or_default::<Vec<serde_json::Value>>(Id::new(
            "editor.icon.paint_records",
        ));
        if records.len() < 4096 {
            records.push(record);
        } else {
            d.insert_temp(Id::new("editor.icon.records_truncated"), true);
        }
    });
    true
}

/// Overlay a reviewed resource on the unchanged semantic clear-search button.
pub(crate) fn search_cancel_button(
    ui: &mut egui::Ui,
    enabled: bool,
    widget_role: &str,
) -> egui::Response {
    if !crate::appearance::is_reference(ui) {
        return ui.add_enabled(enabled, egui::Button::new("×"));
    }
    ui.add_enabled(enabled, |ui: &mut egui::Ui| {
        let response = ui.add(egui::Button::new(
            egui::RichText::new("×").color(egui::Color32::TRANSPARENT),
        ));
        let role = if !enabled {
            "search_cancel_off"
        } else if response.is_pointer_button_down_on() {
            "search_cancel_active"
        } else {
            "search_cancel"
        };
        let rect = egui::Rect::from_center_size(response.rect.center(), egui::vec2(14.0, 14.0));
        let painted = response.rect.contains_rect(rect)
            && ui.clip_rect().contains_rect(rect)
            && paint(ui, role, widget_role, response.id, rect);
        if !painted {
            ui.painter().text(
                response.rect.center(),
                egui::Align2::CENTER_CENTER,
                "×",
                egui::TextStyle::Button.resolve(ui.style()),
                ui.style().interact(&response).text_color(),
            );
        }
        response
    })
}

pub(crate) fn disclosure(ui: &mut egui::Ui, openness: f32, response: &egui::Response) {
    let role = if openness > 0.5 {
        "disclosure_open"
    } else {
        "disclosure_closed"
    };
    let rect = egui::Rect::from_center_size(response.rect.center(), egui::vec2(13.0, 13.0));
    if !paint(ui, role, "inspector.disclosure", response.id, rect) {
        egui::collapsing_header::paint_default_icon(ui, openness, response);
    }
}
