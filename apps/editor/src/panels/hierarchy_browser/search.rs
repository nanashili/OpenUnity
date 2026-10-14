use crate::appearance;
use eframe::egui::{self, Ui};

use super::HierarchyBrowser;

impl HierarchyBrowser {
    pub(crate) fn search(&mut self, ui: &mut Ui) {
        self.search_controls.clear();

        let (search_icon, _) = ui.allocate_exact_size(egui::vec2(8.0, 18.0), egui::Sense::hover());
        let response = ui.add_sized(
            [ui.available_width().max(20.0) - 22.0, 18.0],
            egui::TextEdit::singleline(&mut self.query).hint_text("Search hierarchy"),
        );

        crate::resources::paint(
            ui,
            "search",
            "hierarchy.search",
            response.id,
            egui::Rect::from_center_size(search_icon.center(), egui::vec2(8.0, 8.0)),
        );
        self.search_controls
            .push(appearance::control("hierarchy.search", &response, false));

        let clear =
            crate::resources::search_cancel_button(ui, !self.query.is_empty(), "hierarchy.clear");
        self.search_controls
            .push(appearance::control("hierarchy.clear", &clear, false));

        let mut query_changed = response.changed();
        if clear.clicked() {
            self.query.clear();
            query_changed = true;
        }

        if query_changed {
            self.lowercase_query = self.query.to_lowercase();
            self.search_included = None;
            self.visible_indices = None;
        }
    }
}
