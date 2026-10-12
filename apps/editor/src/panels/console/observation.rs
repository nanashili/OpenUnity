//! Semantic state and actual rendered control observations.

use super::*;

impl ConsolePanel {
    /// Actual Reference view semantics and last rendered widget/scroll geometry.
    pub fn snapshot(&self) -> serde_json::Value {
        let mut value = self.detail.snapshot().to_json();
        value["filter"] = self.filter.clone().into();
        value["enabled_severities"] = self
            .severities()
            .into_iter()
            .map(severity_label)
            .collect::<Vec<_>>()
            .into();
        value["visible_sequences"] = self.visible_sequences.clone().into();
        value["controls"] = self.controls.clone().into();
        value["rows"] = self.rows.clone().into();
        value["severity_icons"] = self.severity_icons.clone().into();
        value["paint_clip"] = self.paint_clip.map(crate::appearance::rect_json).into();
        value["detail_scroll"] = if self.scroll_identity == self.detail.snapshot().selected {
            self.detail_scroll.clone().into()
        } else {
            serde_json::Value::Null
        };
        value
    }

    pub(super) fn record_control(&mut self, role: &str, response: &egui::Response, selected: bool) {
        self.controls.push(serde_json::json!({
            "role": role,
            "rect": crate::appearance::rect_json(response.rect),
            "widget_id": response.id.value(),
            "enabled": response.enabled(),
            "selected": selected,
        }));
    }
}
