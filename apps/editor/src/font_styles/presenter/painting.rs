//! Paint accepted explicit styles and record actual label-family use.

use eframe::egui::{self, FontFamily, FontId};

use super::{FontStylePresenter, ProportionalStyleV1};

impl FontStylePresenter {
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
        let exact = self.exact_style(ui, &font_id, &galley);
        let response = ui.add(egui::Label::new(galley));
        self.record_paint(ui, text, &response, &font_id, exact);
        response
    }
    fn exact_style(
        &self,
        ui: &egui::Ui,
        font_id: &FontId,
        galley: &egui::Galley,
    ) -> Option<ProportionalStyleV1> {
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
                        .all(|section| section.format.font_id == *font_id)
                    && ui.fonts(|fonts| {
                        let defs = fonts.definitions();
                        defs.families
                            .get(&font_id.family)
                            .and_then(|names| names.first())
                            .is_some_and(|first| first == name)
                            && defs
                                .font_data
                                .get(name)
                                .is_some_and(|data| data.index == 0 && data.font.as_ref() == bytes)
                    })
            })
    }

    fn record_paint(
        &mut self,
        ui: &egui::Ui,
        text: &str,
        response: &egui::Response,
        font_id: &FontId,
        exact: Option<ProportionalStyleV1>,
    ) {
        let primary = ui.fonts(|fonts| {
            fonts
                .definitions()
                .families
                .get(&font_id.family)
                .and_then(|names| names.first())
                .cloned()
        });
        self.last_paint = Some(serde_json::json!({
            "text": text,
            "widget_id": response.id.value(),
            "rect": crate::appearance::rect_json(response.rect),
            "requested_style": self.selection.as_ref().and_then(|s| s.requested_style).map(ProportionalStyleV1::name),
            "family": format!("{:?}", font_id.family),
            "logical_size": font_id.size,
            "pixels_per_point": ui.ctx().pixels_per_point(),
            "selected_primary": primary,
            "exact_style_selected": exact.is_some(),
            "asset_sha256": exact.map(|s| s.face().3),
            "face_index": exact.map(|_| 0),
            "actual_fallback": if exact.is_some() { None } else { Some("existing proportional regular/fallback family") },
            "glyph_fallback_and_Unity_raster_equivalence_verified": false
        }));
    }
}
