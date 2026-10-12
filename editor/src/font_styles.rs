//! Explicit, versioned proportional styles from the reviewed Inter face catalog.
//! No existing editor control is assigned a style by this module.

use std::collections::BTreeMap;
use std::sync::{Arc, OnceLock};

use eframe::egui::{self, Context, FontData, FontDefinitions, FontFamily, FontId, Id};

use crate::appearance::AppearanceProfile;

const SEMIBOLD: &[u8] =
    local_reference_bytes!("../assets/unity-7000.0.0a8/fonts/Inter-SemiBold.ttf");
const ITALIC: &[u8] = local_reference_bytes!("../assets/unity-7000.0.0a8/fonts/Inter-Italic.ttf");
const SEMIBOLD_ITALIC: &[u8] =
    local_reference_bytes!("../assets/unity-7000.0.0a8/fonts/Inter-SemiBoldItalic.ttf");

/// Version1 explicit product requests; none implies a Unity control assignment.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProportionalStyleV1 {
    /// The exact reviewed Inter-SemiBold face, whose recorded OS/2 weight is700.
    Semibold,
    /// The exact reviewed Inter-Italic face, without synthetic skew.
    Italic,
    /// The exact reviewed Inter-SemiBoldItalic face, without synthetic transforms.
    SemiboldItalic,
}

impl ProportionalStyleV1 {
    fn index(self) -> usize {
        match self {
            Self::Semibold => 0,
            Self::Italic => 1,
            Self::SemiboldItalic => 2,
        }
    }
    fn name(self) -> &'static str {
        match self {
            Self::Semibold => "semibold",
            Self::Italic => "italic",
            Self::SemiboldItalic => "semibold_italic",
        }
    }
    fn face(self) -> (&'static str, &'static [u8], usize, &'static str) {
        match self {
            Self::Semibold => (
                "Inter-SemiBold",
                SEMIBOLD,
                447_532,
                "f20a40a8443df2e20a547d31187fb747d32263854a19496e41d88f4204082caa",
            ),
            Self::Italic => (
                "Inter-Italic",
                ITALIC,
                439_620,
                "c630b1bb55a96032d0dae52cb6b4e1a9bef8c179398a5df510eb1c767f6c3500",
            ),
            Self::SemiboldItalic => (
                "Inter-SemiBoldItalic",
                SEMIBOLD_ITALIC,
                453_204,
                "a4ca1a33406802e20bac217b0e5ccb25037e68f396392cf7b8cafb64c0933d74",
            ),
        }
    }
    fn family(self) -> FontFamily {
        FontFamily::Name(format!("OpenUnity.InterStyle.v1.{}", self.name()).into())
    }
}

/// Bounded exact-face availability used by the explicit style renderer.
/// Candidate data can only resolve to the reviewed embedded originals.
#[derive(Clone)]
pub struct FontStyleCatalog {
    faces: [Result<Arc<FontData>, String>; 3],
}

impl FontStyleCatalog {
    /// Validate the reviewed embedded catalog once, without registering or selecting it.
    pub fn embedded() -> Self {
        static CATALOG: OnceLock<FontStyleCatalog> = OnceLock::new();
        CATALOG
            .get_or_init(|| {
                if cfg!(feature = "local-reference-assets") {
                    Self::from_faces(Some(SEMIBOLD), Some(ITALIC), Some(SEMIBOLD_ITALIC))
                } else {
                    Self::from_faces(None, None, None)
                }
            })
            .clone()
    }
    /// Resolve supplied optional face resources against the exact reviewed originals.
    /// Missing, oversized, foreign or unparseable data remains an honest resource failure.
    pub fn from_faces(
        semibold: Option<&[u8]>,
        italic: Option<&[u8]>,
        semibold_italic: Option<&[u8]>,
    ) -> Self {
        let inputs = [semibold, italic, semibold_italic];
        let styles = [
            ProportionalStyleV1::Semibold,
            ProportionalStyleV1::Italic,
            ProportionalStyleV1::SemiboldItalic,
        ];
        Self {
            faces: std::array::from_fn(|index| {
                let style = styles[index];
                let (name, original, expected, _) = style.face();
                let bytes=inputs[index].ok_or_else(||format!("Reviewed font {name} is unavailable; using the existing regular/fallback family."))?;
                if bytes.len() != expected || bytes.len() > 1_048_576 || bytes != original {
                    return Err(format!(
                        "Reviewed font {name} failed exact resource validation; using the existing regular/fallback family."
                    ));
                }
                let data = Arc::new(FontData::from_static(original));
                let mut probe = FontDefinitions::empty();
                probe.font_data.insert(name.into(), data.clone());
                if std::panic::catch_unwind(|| {
                    egui::epaint::text::Fonts::new(1024, Default::default(), probe)
                })
                .is_err()
                {
                    return Err(format!(
                        "Reviewed font {name} could not be parsed; using the existing regular/fallback family."
                    ));
                }
                Ok(data)
            }),
        }
    }
}

type RegisteredFaces = BTreeMap<String, Arc<FontData>>;

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

/// Last accepted request, distinct from proof that a live label selected its face.
#[derive(Clone, Debug, PartialEq)]
pub struct FontStyleSelection {
    /// Profile at the accepted request boundary.
    pub profile: AppearanceProfile,
    /// Explicit requested style, or unchanged regular text.
    pub requested_style: Option<ProportionalStyleV1>,
    /// Requested positive finite logical text size.
    pub logical_size: f32,
    /// Requested positive finite renderer scale; no scale-specific font is substituted.
    pub pixels_per_point: f32,
    /// Accepted optional family, or the existing proportional fallback.
    pub font_id: FontId,
}

impl FontStyleSelection {
    /// Describe a request without claiming it was rendered or selected.
    pub fn to_json(&self) -> serde_json::Value {
        serde_json::json!({"profile":self.profile.name(),"requested_style":self.requested_style.map(ProportionalStyleV1::name),
            "logical_size":self.logical_size,"pixels_per_point":self.pixels_per_point,"family":format!("{:?}",self.font_id.family)})
    }
}

/// Explicit style request and actual semantic Label renderer, independent of document state.
pub struct FontStylePresenter {
    catalog: FontStyleCatalog,
    selection: Option<FontStyleSelection>,
    diagnostic: Option<String>,
    last_paint: Option<serde_json::Value>,
}

impl Default for FontStylePresenter {
    fn default() -> Self {
        Self::new(FontStyleCatalog::embedded())
    }
}

impl FontStylePresenter {
    /// Create a renderer with a bounded exact-face availability catalog.
    pub fn new(catalog: FontStyleCatalog) -> Self {
        Self {
            catalog,
            selection: None,
            diagnostic: None,
            last_paint: None,
        }
    }
    /// Validate before any registration or accepted-presentation update.
    /// Invalid geometry preserves the entire prior request and paint receipt.
    pub fn request(
        &mut self,
        ctx: &Context,
        profile: AppearanceProfile,
        style: Option<ProportionalStyleV1>,
        logical_size: f32,
        pixels_per_point: f32,
    ) -> Result<FontStyleSelection, String> {
        if !logical_size.is_finite()
            || logical_size <= 0.0
            || !pixels_per_point.is_finite()
            || pixels_per_point <= 0.0
        {
            let error = "Invalid explicit font style size or display scale".to_owned();
            self.diagnostic = Some(error.clone());
            return Err(error);
        }
        let mut family = FontFamily::Proportional;
        let mut diagnostic = None;
        if profile == AppearanceProfile::Reference
            && let Some(style) = style
        {
            match &self.catalog.faces[style.index()] {
                Ok(data) => {
                    let (name, _, _, _) = style.face();
                    ctx.data_mut(|d| {
                        d.get_temp_mut_or_default::<RegisteredFaces>(Id::new(
                            "editor.font_styles.v1.registered",
                        ))
                        .insert(name.into(), data.clone())
                    });
                    family = style.family();
                }
                Err(error) => diagnostic = Some(error.clone()),
            }
        }
        if let Some(error) = crate::resources::install(ctx, profile) {
            diagnostic = Some(error);
            family = FontFamily::Proportional;
        }
        let selection = FontStyleSelection {
            profile,
            requested_style: style,
            logical_size,
            pixels_per_point,
            font_id: FontId::new(logical_size, family),
        };
        self.selection = Some(selection.clone());
        self.diagnostic = diagnostic;
        Ok(selection)
    }
    /// Last accepted request; invalid requests leave it unchanged.
    pub fn selection(&self) -> Option<&FontStyleSelection> {
        self.selection.as_ref()
    }
    /// Current rejection/resource diagnostic, separate from the retained presentation.
    pub fn diagnostic(&self) -> Option<&str> {
        self.diagnostic.as_deref()
    }
    /// Paint a real Label using only the accepted explicit request, retaining its text semantics.
    /// Selection receipts require current definitions and the actual laid-out galley family.
    pub fn label(&mut self, ui: &mut egui::Ui, text: &str) -> egui::Response {
        let mut font_id = self
            .selection
            .as_ref()
            .map(|s| s.font_id.clone())
            .unwrap_or_else(|| egui::TextStyle::Body.resolve(ui.style()));
        if !crate::appearance::is_reference(ui)
            || !ui.fonts(|fonts| fonts.definitions().families.contains_key(&font_id.family))
        {
            font_id.family = FontFamily::Proportional;
        }
        let galley = egui::WidgetText::from(egui::RichText::new(text).font(font_id.clone()))
            .into_galley(ui, None, ui.available_width(), egui::FontSelection::Default);
        let exact =
            self.selection
                .as_ref()
                .and_then(|selection| selection.requested_style)
                .filter(|style| {
                    let (name, bytes, _, _) = style.face();
                    crate::appearance::is_reference(ui)
                        && font_id.family == style.family()
                        && galley
                            .job
                            .sections
                            .iter()
                            .all(|section| section.format.font_id == font_id)
                        && ui.fonts(|fonts| {
                            let defs = fonts.definitions();
                            defs.families
                                .get(&font_id.family)
                                .and_then(|names| names.first())
                                .is_some_and(|first| first == name)
                                && defs.font_data.get(name).is_some_and(|data| {
                                    data.index == 0 && data.font.as_ref() == bytes
                                })
                        })
                });
        let response = ui.add(egui::Label::new(galley));
        let primary = ui.fonts(|fonts| {
            fonts
                .definitions()
                .families
                .get(&font_id.family)
                .and_then(|names| names.first())
                .cloned()
        });
        self.last_paint = Some(
            serde_json::json!({"text":text,"widget_id":response.id.value(),"rect":crate::appearance::rect_json(response.rect),
            "requested_style":self.selection.as_ref().and_then(|s|s.requested_style).map(ProportionalStyleV1::name),
            "family":format!("{:?}",font_id.family),"logical_size":font_id.size,"pixels_per_point":ui.ctx().pixels_per_point(),
            "selected_primary":primary,"exact_style_selected":exact.is_some(),"asset_sha256":exact.map(|s|s.face().3),"face_index":exact.map(|_|0),
            "actual_fallback":if exact.is_some(){None}else{Some("existing proportional regular/fallback family")},
            "glyph_fallback_and_Unity_raster_equivalence_verified":false}),
        );
        response
    }
    /// Separate accepted request, diagnostic and actual last-label use evidence.
    pub fn snapshot(&self) -> serde_json::Value {
        serde_json::json!({"selection":self.selection.as_ref().map(FontStyleSelection::to_json),"diagnostic":self.diagnostic,"last_paint":self.last_paint,
            "local_reference_assets_enabled":cfg!(feature = "local-reference-assets"),
            "manifest_sha256":cfg!(feature = "local-reference-assets").then_some("0174245c48a5e8bc6b2cfaa8872f92b17883fc13e453c8166ebce38b9f114d7d")})
    }
}
