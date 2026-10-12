//! Explicit style acceptance, semantic label painting, and use receipts.

use eframe::egui::{Context, FontFamily, FontId, Id};

use crate::appearance::AppearanceProfile;

use super::{FontStyleCatalog, FontStylePresenter, FontStyleSelection, ProportionalStyleV1};

mod painting;

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
        let (family, diagnostic) = self.resolve_family(ctx, profile, style);
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
    fn resolve_family(
        &self,
        ctx: &Context,
        profile: AppearanceProfile,
        style: Option<ProportionalStyleV1>,
    ) -> (FontFamily, Option<String>) {
        let mut family = FontFamily::Proportional;
        let mut diagnostic = None;
        if profile == AppearanceProfile::Reference
            && let Some(style) = style
        {
            match &self.catalog.faces[style.index()] {
                Ok(data) => {
                    let (name, _, _, _) = style.face();
                    ctx.data_mut(|d| {
                        d.get_temp_mut_or_default::<super::registration::RegisteredFaces>(Id::new(
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
        (family, diagnostic)
    }

    /// Separate accepted request, diagnostic and actual last-label use evidence.
    pub fn snapshot(&self) -> serde_json::Value {
        serde_json::json!({
            "selection": self.selection.as_ref().map(FontStyleSelection::to_json),
            "diagnostic": self.diagnostic,
            "last_paint": self.last_paint,
            "local_reference_assets_enabled": cfg!(feature = "local-reference-assets"),
            "manifest_sha256": cfg!(feature = "local-reference-assets").then_some("0174245c48a5e8bc6b2cfaa8872f92b17883fc13e453c8166ebce38b9f114d7d")
        })
    }
}
