//! Read-only creation affordance and the existing Reference search controls.

use super::*;

impl ProjectBrowserPanel {
    pub(super) fn show_reference_toolbar(
        &mut self,
        ui: &mut egui::Ui,
        project: Option<&Project>,
        frame: &mut ReferenceFrame,
    ) {
        let toolbar_rect =
            egui::Rect::from_min_size(ui.cursor().min, egui::vec2(ui.available_width(), 20.0));
        ui.painter().rect_filled(toolbar_rect, 0.0, appearance::BAR);
        ui.horizontal(|ui| {
            Self::show_create_control(ui, frame);
            if let Some(project) = project {
                self.show_reference_source_controls(ui, project, frame);
                let active_source_label = source::label(project, &self.source_key);
                ui.label(&active_source_label).on_hover_text(format!(
                    "{}\nDeclared editor: {}\nAssets discovery diagnostics: {}\nRead-only source snapshot; files have not been imported.",
                    project.root().display(),
                    project.editor_version(),
                    project.diagnostics().len(),
                ));
                if let Some(active_source) = source::resolve(project, &self.source_key)
                    && let Some(warning) = source::incomplete_warning(active_source)
                {
                    ui.colored_label(egui::Color32::YELLOW, "Source may be incomplete")
                        .on_hover_text(warning);
                }
            } else {
                ui.weak("No project");
            }
            self.show_search_controls(ui, frame);
        });
    }

    fn show_create_control(ui: &mut egui::Ui, frame: &mut ReferenceFrame) {
        // add_enabled disables this same UI, preserving the original button ID
        // and applying the ordinary disabled painter to the exact resource.
        let create = ui
            .add_enabled(false, |ui: &mut egui::Ui| {
                let response = ui.add(
                    egui::Button::new(egui::RichText::new("+").color(egui::Color32::TRANSPARENT))
                        .min_size(egui::vec2(16.0, 0.0)),
                );
                let icon_rect =
                    egui::Rect::from_center_size(response.rect.center(), egui::vec2(16.0, 16.0));
                let painted = crate::resources::paint(
                    ui,
                    "project_create",
                    "project.create",
                    response.id,
                    icon_rect,
                );
                if !painted {
                    ui.painter().text(
                        response.rect.center(),
                        egui::Align2::CENTER_CENTER,
                        "+",
                        egui::TextStyle::Button.resolve(ui.style()),
                        ui.style().interact(&response).text_color(),
                    );
                }
                frame.create_icon = serde_json::json!({
                    "widget_id": response.id.value(),
                    "button_rect": appearance::rect_json(response.rect),
                    "icon_rect": appearance::rect_json(icon_rect),
                    "paint_success": painted,
                    "enabled": response.enabled()
                });
                response
            })
            .on_disabled_hover_text(
                "Asset creation and import are unsupported. This browser is read-only.",
            );
        frame
            .controls
            .push(appearance::control("project.create", &create, false));
    }

    fn show_search_controls(&mut self, ui: &mut egui::Ui, frame: &mut ReferenceFrame) {
        let hint = source::reference_search_hint(&self.source_key);
        let source_scope = source::source_role(&self.source_key);
        ui.push_id(source_scope, |ui| {
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let clear = crate::resources::search_cancel_button(
                    ui,
                    !self.filter.is_empty(),
                    "project.clear",
                )
                .on_hover_text("Clear search");
                frame
                    .controls
                    .push(appearance::control("project.clear", &clear, false));
                if clear.clicked() {
                    self.filter.clear();
                }

                // Reserve the existing8px search icon and ordinary item gap.
                // These are layout requests; the receipt retains the actual response.
                let available = ui.available_rect_before_wrap();
                let icon_reservation = 8.0 + ui.spacing().item_spacing.x;
                let available_width = (available.width() - icon_reservation).max(0.0);
                let requested_width = available_width.clamp(65.0, 300.0).min(available_width);
                let requested_rect = egui::Rect::from_min_max(
                    egui::pos2(available.right() - requested_width, available.top()),
                    egui::pos2(available.right(), available.top() + 18.0),
                );
                let search = ui.add_sized(
                    [requested_width, 18.0],
                    egui::TextEdit::singleline(&mut self.filter)
                        .hint_text(hint)
                        .desired_width(requested_width),
                );
                let (icon, _) = ui.allocate_exact_size(egui::vec2(8.0, 18.0), egui::Sense::hover());
                crate::resources::paint(
                    ui,
                    "search",
                    "project.search",
                    search.id,
                    egui::Rect::from_center_size(icon.center(), egui::vec2(8.0, 8.0)),
                );
                frame.search_geometry = serde_json::json!({
                    "role": "project.search",
                    "widget_id": search.id.value(),
                    "requested": {"height": 18.0, "min_width": 65.0, "max_width": 300.0},
                    "available_rect": appearance::rect_json(egui::Rect::from_min_max(
                        egui::pos2(available.left() + icon_reservation, available.top()),
                        available.max
                    )),
                    "requested_rect": appearance::rect_json(requested_rect),
                    "actual_rect": appearance::rect_json(search.rect)
                });
                frame
                    .controls
                    .push(appearance::control("project.search", &search, false));
            });
        });
    }
}
