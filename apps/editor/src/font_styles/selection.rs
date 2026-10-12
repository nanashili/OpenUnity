//! Accepted presentation request, separate from actual painted-label evidence.

use eframe::egui::FontId;

use crate::appearance::AppearanceProfile;

use super::ProportionalStyleV1;

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
        serde_json::json!({
            "profile": self.profile.name(),
            "requested_style": self.requested_style.map(ProportionalStyleV1::name),
            "logical_size": self.logical_size,
            "pixels_per_point": self.pixels_per_point,
            "family": format!("{:?}", self.font_id.family)
        })
    }
}
