//! Independently authored controls for OpenUnity editor events, without Unity log capture.

use crate::console_detail::{
    ConsoleDetailController, ConsoleLifetime, ConsoleRecordIdentity, severity_label,
};
use eframe::egui::{self, Label, ScrollArea, TextEdit, TextStyle, Ui};
use openunity_editor_core::console::{Console, Severity};

/// Severity and literal-message filters for the live editor console.
pub struct ConsolePanel {
    info: bool,
    warning: bool,
    error: bool,
    filter: String,
    detail: ConsoleDetailController,
    controls: Vec<serde_json::Value>,
    rows: Vec<serde_json::Value>,
    severity_icons: Vec<serde_json::Value>,
    visible_sequences: Vec<u64>,
    paint_clip: Option<egui::Rect>,
    detail_scroll: Option<serde_json::Value>,
    scroll_identity: Option<ConsoleRecordIdentity>,
}

impl Default for ConsolePanel {
    fn default() -> Self {
        Self {
            info: true,
            warning: true,
            error: true,
            filter: String::new(),
            detail: ConsoleDetailController::default(),
            controls: Vec::new(),
            rows: Vec::new(),
            severity_icons: Vec::new(),
            visible_sequences: Vec::new(),
            paint_clip: None,
            detail_scroll: None,
            scroll_identity: None,
        }
    }
}

impl ConsolePanel {
    /// Displays retained records and applies clear directly to the supplied console.
    pub fn show(&mut self, ui: &mut Ui, console: &mut Console) {
        if crate::appearance::is_reference(ui) {
            self.show_reference(ui, console);
            return;
        }
        // Legacy renders none of the Reference record/detail controls. Retain
        // semantic selection and its offset for a later Reference frame.
        self.controls.clear();
        self.severity_icons.clear();
        self.rows.clear();
        self.paint_clip = None;
        self.detail_scroll = None;
        self.scroll_identity = None;
        let compact = crate::appearance::is_reference(ui);
        ui.horizontal_wrapped(|ui| {
            ui.checkbox(&mut self.info, "Info");
            ui.checkbox(&mut self.warning, "Warning");
            ui.checkbox(&mut self.error, "Error");
            if ui
                .add_enabled(
                    console.records().len() != 0,
                    eframe::egui::Button::new("Clear"),
                )
                .clicked()
            {
                console.clear();
            }
        });
        ui.horizontal(|ui| {
            let label = ui.label("Filter messages");
            ui.add(TextEdit::singleline(&mut self.filter).hint_text("Search message text"))
                .labelled_by(label.id)
                .on_hover_text("Literal, case-sensitive message substring");
        });
        let severities: Vec<_> = [
            (self.info, Severity::Info),
            (self.warning, Severity::Warning),
            (self.error, Severity::Error),
        ]
        .into_iter()
        .filter_map(|(enabled, severity)| enabled.then_some(severity))
        .collect();
        let records: Vec<_> = console.filtered(&severities, &self.filter).collect();
        let summary = format!(
            "Showing {} of {} retained records · Dropped records: {}",
            records.len(),
            console.records().len(),
            console.dropped_count()
        );
        if compact {
            ui.small(summary);
        } else {
            ui.label(summary);
        }
        ui.separator();
        if records.is_empty() {
            ui.label(if console.records().len() == 0 {
                "No editor records."
            } else {
                "No records match the current filters."
            });
        }
        // Console retention bounds this list to 1024 entries. Render only visible
        // fixed-height rows; a hovered row reveals its unmodified full message.
        let row_height = ui.text_style_height(&TextStyle::Body);
        ScrollArea::vertical().id_salt("console_records").show_rows(
            ui,
            row_height,
            records.len(),
            |ui, range| {
                for record in &records[range] {
                    let severity = match record.severity {
                        Severity::Info => "Info",
                        Severity::Warning => "Warning",
                        Severity::Error => "Error",
                    };
                    ui.push_id(record.sequence, |ui| {
                        ui.add(
                            Label::new(format!(
                                "#{} [{severity}] {}",
                                record.sequence, record.message
                            ))
                            .truncate(),
                        )
                        .on_hover_text(&record.message);
                    });
                }
            },
        );
    }
    fn severities(&self) -> Vec<Severity> {
        [
            (self.info, Severity::Info),
            (self.warning, Severity::Warning),
            (self.error, Severity::Error),
        ]
        .into_iter()
        .filter_map(|(enabled, severity)| enabled.then_some(severity))
        .collect()
    }
    /// Current explicit owner scope, independent of records and storage addresses.
    pub fn lifetime(&self) -> Option<ConsoleLifetime> {
        self.detail.lifetime()
    }
    /// Capture a current owner with an existing record sequence for later validation.
    pub fn identity(&self, sequence: u64) -> Option<ConsoleRecordIdentity> {
        self.detail.identity(sequence)
    }
    /// Rebind before supplying a replaced Console, invalidating every old-scope event.
    pub fn reset_owner(&mut self) -> Result<(), String> {
        self.detail.reset_owner()?;
        self.controls.clear();
        self.severity_icons.clear();
        self.rows.clear();
        self.visible_sequences.clear();
        self.detail_scroll = None;
        self.scroll_identity = None;
        self.paint_clip = None;
        Ok(())
    }
    /// Reconcile current retention and filters without selecting any fallback record.
    pub fn reconcile(&mut self, console: &Console) {
        let severities = self.severities();
        self.detail.reconcile(console, &severities, &self.filter);
        self.visible_sequences = console
            .filtered(&severities, &self.filter)
            .map(|r| r.sequence)
            .collect();
    }
    /// Resolve a captured visible record against the current Console and filters.
    pub fn select(
        &mut self,
        identity: ConsoleRecordIdentity,
        console: &Console,
    ) -> Result<(), String> {
        let severities = self.severities();
        self.detail
            .select(identity, console, &severities, &self.filter)
    }
    /// Replace the existing literal, case-sensitive message filter and reconcile detail.
    pub fn set_filter(&mut self, filter: String, console: &Console) {
        self.reconcile(console);
        self.filter = filter;
        self.reconcile(console);
    }
    /// Replace enabled severity filters using the existing severity-intersection semantics.
    pub fn set_severities(&mut self, severities: &[Severity], console: &Console) {
        self.reconcile(console);
        self.info = severities.contains(&Severity::Info);
        self.warning = severities.contains(&Severity::Warning);
        self.error = severities.contains(&Severity::Error);
        self.reconcile(console);
    }
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
    fn record_control(&mut self, role: &str, response: &egui::Response, selected: bool) {
        self.controls.push(serde_json::json!({"role":role,"rect":crate::appearance::rect_json(response.rect),"widget_id":response.id.value(),"enabled":response.enabled(),"selected":selected}));
    }
    fn severity_checkbox(
        ui: &mut Ui,
        checked: &mut bool,
        label: &str,
        resource_role: &str,
        retained_count: usize,
    ) -> (egui::Response, serde_json::Value) {
        let galley = egui::WidgetText::from(label).into_galley(
            ui,
            Some(egui::TextWrapMode::Extend),
            f32::INFINITY,
            egui::FontSelection::Default,
        );
        let checkbox_width =
            (ui.spacing().icon_width + ui.spacing().icon_spacing + galley.size().x)
                .max(ui.spacing().interact_size.y);
        // Wrap the entire decorative pair without creating a child UI or another
        // widget ID. The checkbox remains the existing semantic/focus control.
        if ui.available_rect_before_wrap().width() < checkbox_width + 18.0
            && ui.cursor().left() > ui.max_rect().left()
        {
            ui.end_row();
        }
        let response = ui.checkbox(checked, label);
        let icon_rect = egui::Rect::from_min_size(
            egui::pos2(response.rect.right() + 2.0, response.rect.center().y - 8.0),
            egui::vec2(16.0, 16.0),
        );
        ui.add_space(18.0);
        let role = format!("console.severity.{label}");
        let painted = crate::resources::paint_counted(
            ui,
            resource_role,
            &role,
            response.id,
            icon_rect,
            retained_count,
        );
        let geometry = serde_json::json!({"role":role,"widget_id":response.id.value(),"severity":label,
            "resource_role":resource_role,"retained_count":retained_count,"presentation_state":if retained_count == 0 {"zero_count"} else {"positive_count"},
            "checkbox_rect":crate::appearance::rect_json(response.rect),"icon_rect":crate::appearance::rect_json(icon_rect),
            "group_rect":crate::appearance::rect_json(response.rect.union(icon_rect)),"paint_success":painted});
        (response, geometry)
    }
    fn show_reference(&mut self, ui: &mut Ui, console: &mut Console) {
        self.controls.clear();
        self.severity_icons.clear();
        self.reconcile(console);
        let mut counts = [0usize; 3];
        for record in console.records() {
            counts[match record.severity {
                Severity::Info => 0,
                Severity::Warning => 1,
                Severity::Error => 2,
            }] += 1;
        }
        ui.horizontal_wrapped(|ui| {
            let (response, icon) =
                Self::severity_checkbox(ui, &mut self.info, "Info", "console_info", counts[0]);
            self.record_control("console.severity.Info", &response, self.info);
            self.severity_icons.push(icon);
            let (response, icon) = Self::severity_checkbox(
                ui,
                &mut self.warning,
                "Warning",
                "console_warning",
                counts[1],
            );
            self.record_control("console.severity.Warning", &response, self.warning);
            self.severity_icons.push(icon);
            let (response, icon) =
                Self::severity_checkbox(ui, &mut self.error, "Error", "console_error", counts[2]);
            self.record_control("console.severity.Error", &response, self.error);
            self.severity_icons.push(icon);
            let response = ui.add_enabled(console.records().len() != 0, egui::Button::new("Clear"));
            self.record_control("console.clear", &response, false);
            if response.clicked() {
                console.clear();
                self.reconcile(console);
            }
        });
        ui.horizontal(|ui| {
            let label = ui.label("Filter messages");
            let response = ui
                .add(TextEdit::singleline(&mut self.filter).hint_text("Search message text"))
                .labelled_by(label.id)
                .on_hover_text("Literal, case-sensitive message substring");
            self.record_control("console.filter", &response, false);
        });
        self.reconcile(console);
        ui.small(format!(
            "Showing {} of {} retained records · Dropped records: {}",
            self.visible_sequences.len(),
            console.records().len(),
            console.dropped_count()
        ));
        ui.separator();
        self.show_remaining(ui, console, ui.available_rect_before_wrap());
    }
    /// Render the real Reference list/detail subdivision after the unchanged header controls.
    /// This boundary also supports a genuinely constrained remaining pane without new docking.
    pub fn show_remaining(
        &mut self,
        ui: &mut Ui,
        console: &mut Console,
        remaining_rect: egui::Rect,
    ) {
        self.reconcile(console);
        self.rows.clear();
        self.controls.retain(|control| {
            control["role"]
                .as_str()
                .is_some_and(|role| !role.starts_with("console.row.") && role != "console.detail")
        });
        let (layout, valid_layout) = match self.detail.layout(remaining_rect) {
            Ok(layout) => (layout, true),
            Err(error) => {
                ui.response().on_hover_text(error);
                let Some(layout) = self.detail.snapshot().layout else {
                    self.paint_clip = None;
                    self.detail_scroll = None;
                    self.scroll_identity = None;
                    return;
                };
                (layout, false)
            }
        };
        // A rejected measurement cannot replace the last valid presentation.
        // Its retained regions still obey the current actual pane's paint clip.
        let clip = if valid_layout {
            remaining_rect.intersect(ui.clip_rect())
        } else {
            ui.clip_rect()
        };
        self.paint_clip = Some(clip);
        let lifetime = self.lifetime().map(ConsoleLifetime::value).unwrap_or(0);
        let list_clip = layout.list_rect.intersect(clip);
        if list_clip.width() > 0.0 && list_clip.height() > 0.0 {
            let mut list_ui = ui.new_child(
                egui::UiBuilder::new()
                    .id_salt(("console.list", lifetime))
                    .max_rect(layout.list_rect),
            );
            list_ui.set_clip_rect(list_clip);
            let severities = self.severities();
            let filter = self.filter.clone();
            let records: Vec<_> = console.filtered(&severities, &filter).collect();
            if records.is_empty() {
                list_ui.label(if console.records().len() == 0 {
                    "No editor records."
                } else {
                    "No records match the current filters."
                });
            } else {
                let row_height = list_ui.text_style_height(&TextStyle::Body);
                list_ui.spacing_mut().interact_size.y = row_height;
                list_ui.spacing_mut().button_padding.y = 0.0;
                ScrollArea::vertical().id_salt(("console_records",lifetime)).auto_shrink([false,false]).min_scrolled_height(0.0).max_height(layout.list_rect.height()).show_rows(&mut list_ui,row_height,records.len(),|ui,range| {
                    for record in &records[range] {
                        let Some(identity) = self.identity(record.sequence) else { continue; };
                        let selected = self.detail.snapshot().selected == Some(identity);
                        let text = format!("#{} [{}] {}",record.sequence,severity_label(record.severity),record.message);
                        let response = ui.push_id(("console.row",lifetime,record.sequence),|ui| {
                            ui.add_sized([ui.available_width(),row_height],egui::Button::new(&text).truncate().selected(selected).fill(if selected {crate::appearance::SELECTION} else {egui::Color32::TRANSPARENT}).stroke(egui::Stroke::NONE))
                        }).inner;
                        response.widget_info(||egui::WidgetInfo::selected(egui::WidgetType::SelectableLabel,response.enabled(),selected,&text));
                        let response = response.on_hover_text(&record.message);
                        self.record_control(&format!("console.row.{}",record.sequence),&response,selected);
                        self.rows.push(serde_json::json!({"sequence":record.sequence,"rect":crate::appearance::rect_json(response.rect),"widget_id":response.id.value(),"selected":selected}));
                        if response.clicked() {
                            let _ = self.detail.select(identity,console,&severities,&self.filter);
                            ui.ctx().request_repaint();
                        }
                    }
                });
            }
        }
        // Validate again immediately before detail paint, including zero-size allocations.
        self.reconcile(console);
        let state = self.detail.snapshot().clone();
        let semantic_text = match (&state.detail_message, state.detail_severity) {
            (Some(message), Some(severity)) => format!("[{}] {message}", severity_label(severity)),
            _ => "Select a console record to view its full message.".into(),
        };
        let semantic_response = ui.interact(
            layout.detail_rect,
            ui.make_persistent_id(("console.detail.semantic", lifetime)),
            egui::Sense::hover(),
        );
        semantic_response.widget_info(|| {
            egui::WidgetInfo::labeled(egui::WidgetType::Label, ui.is_enabled(), &semantic_text)
        });
        self.record_control("console.detail", &semantic_response, false);
        if let Some(error) = &state.diagnostic {
            semantic_response.on_hover_text(error);
        }
        let detail_clip = layout.detail_rect.intersect(clip);
        if detail_clip.width() <= 0.0 || detail_clip.height() <= 0.0 {
            return;
        }
        let mut detail_ui = ui.new_child(
            egui::UiBuilder::new()
                .id_salt(("console.detail", lifetime))
                .max_rect(layout.detail_rect),
        );
        detail_ui.set_clip_rect(detail_clip);
        let scroll = ScrollArea::vertical()
            .id_salt(("console_detail", lifetime))
            .auto_shrink([false, false])
            .min_scrolled_height(0.0)
            .max_height(layout.detail_rect.height())
            .vertical_scroll_offset(state.detail_scroll_y)
            .show(&mut detail_ui, |ui| {
                if let (Some(message), Some(severity)) =
                    (&state.detail_message, state.detail_severity)
                {
                    ui.label(severity_label(severity));
                    ui.add(Label::new(message).wrap());
                } else {
                    ui.add(
                        Label::new(
                            state
                                .diagnostic
                                .as_deref()
                                .unwrap_or("Select a console record to view its full message."),
                        )
                        .wrap(),
                    );
                }
            });
        let max_scroll_y = (scroll.content_size.y - scroll.inner_rect.height()).max(0.0);
        if valid_layout {
            let _ = self
                .detail
                .update_scroll(scroll.state.offset.y, max_scroll_y);
        }
        self.scroll_identity = state.selected;
        self.detail_scroll = Some(
            serde_json::json!({"inner_rect":crate::appearance::rect_json(scroll.inner_rect),"content_rect":crate::appearance::rect_json(egui::Rect::from_min_size(scroll.inner_rect.min-scroll.state.offset,scroll.content_size)),"offset":[scroll.state.offset.x,scroll.state.offset.y],"max_scroll_y":max_scroll_y}),
        );
    }
}

#[cfg(test)]
mod tests {
    use eframe::egui::{
        CentralPanel, Context, Event, FullOutput, Modifiers, PointerButton, Pos2, RawInput, Rect,
        Vec2, epaint::Shape,
    };

    use super::*;

    fn frame(
        ctx: &Context,
        panel: &mut ConsolePanel,
        console: &mut Console,
        events: Vec<Event>,
    ) -> FullOutput {
        ctx.run(
            RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(800.0, 600.0))),
                events,
                ..Default::default()
            },
            |ctx| {
                CentralPanel::default().show(ctx, |ui| panel.show(ui, console));
            },
        )
    }

    fn text_rows(output: &FullOutput) -> Vec<(String, Pos2)> {
        output
            .shapes
            .iter()
            .filter_map(|shape| match &shape.shape {
                Shape::Text(text) => Some((
                    text.galley.text().to_owned(),
                    text.pos + text.galley.size() * 0.5,
                )),
                _ => None,
            })
            .collect()
    }

    fn click(
        ctx: &Context,
        panel: &mut ConsolePanel,
        console: &mut Console,
        output: &FullOutput,
        label: &str,
    ) -> FullOutput {
        let pos = text_rows(output)
            .into_iter()
            .find_map(|(text, pos)| (text == label).then_some(pos))
            .unwrap_or_else(|| panic!("missing visible control: {label}"));
        frame(
            ctx,
            panel,
            console,
            vec![
                Event::PointerMoved(pos),
                Event::PointerButton {
                    pos,
                    button: PointerButton::Primary,
                    pressed: true,
                    modifiers: Modifiers::NONE,
                },
                Event::PointerButton {
                    pos,
                    button: PointerButton::Primary,
                    pressed: false,
                    modifiers: Modifiers::NONE,
                },
            ],
        )
    }

    fn visible_records(output: &FullOutput) -> Vec<String> {
        text_rows(output)
            .into_iter()
            .map(|(text, _)| text)
            .filter(|text| text.starts_with('#'))
            .collect()
    }

    #[test]
    fn pointer_severity_and_typed_message_filters_preserve_live_record_order() {
        let mut console = Console::new(8).unwrap();
        console.push(Severity::Info, "asset opened").unwrap();
        console.push(Severity::Warning, "scene changed").unwrap();
        console.push(Severity::Error, "asset unavailable").unwrap();
        console.push(Severity::Warning, "asset stale").unwrap();
        let mut panel = ConsolePanel::default();
        let ctx = Context::default();
        let output = frame(&ctx, &mut panel, &mut console, Vec::new());
        assert_eq!(
            visible_records(&output),
            [
                "#1 [Info] asset opened",
                "#2 [Warning] scene changed",
                "#3 [Error] asset unavailable",
                "#4 [Warning] asset stale",
            ]
        );
        let output = click(&ctx, &mut panel, &mut console, &output, "Info");
        assert_eq!(visible_records(&output).len(), 3);
        let _ = click(
            &ctx,
            &mut panel,
            &mut console,
            &output,
            "Search message text",
        );
        let output = frame(
            &ctx,
            &mut panel,
            &mut console,
            vec![Event::Text("asset".to_owned())],
        );
        assert_eq!(
            visible_records(&output),
            ["#3 [Error] asset unavailable", "#4 [Warning] asset stale"]
        );
        let output = click(&ctx, &mut panel, &mut console, &output, "Warning");
        assert_eq!(visible_records(&output), ["#3 [Error] asset unavailable"]);
        let output = click(&ctx, &mut panel, &mut console, &output, "Error");
        assert!(visible_records(&output).is_empty());
        assert!(
            text_rows(&output)
                .iter()
                .any(|(text, _)| text == "No records match the current filters.")
        );
        assert_eq!(console.records().len(), 4);
        assert_eq!(console.dropped_count(), 0);
    }

    #[test]
    fn clear_button_removes_actual_records_and_preserves_visible_overflow() {
        let mut console = Console::new(1).unwrap();
        console.push(Severity::Info, "evicted").unwrap();
        console.push(Severity::Error, "retained").unwrap();
        let mut panel = ConsolePanel::default();
        let ctx = Context::default();
        let output = frame(&ctx, &mut panel, &mut console, Vec::new());
        assert_eq!(visible_records(&output), ["#2 [Error] retained"]);
        assert!(
            text_rows(&output)
                .iter()
                .any(|(text, _)| text.contains("Dropped records: 1"))
        );
        let output = click(&ctx, &mut panel, &mut console, &output, "Error");
        assert!(visible_records(&output).is_empty());
        assert_eq!(console.records().len(), 1);
        let output = click(&ctx, &mut panel, &mut console, &output, "Clear");
        assert_eq!(console.records().len(), 0);
        assert_eq!(console.dropped_count(), 1);
        assert!(visible_records(&output).is_empty());
        assert!(
            text_rows(&output)
                .iter()
                .any(|(text, _)| text == "No editor records.")
        );
        assert!(
            text_rows(&output)
                .iter()
                .any(|(text, _)| text.contains("Dropped records: 1"))
        );
        assert_eq!(console.push(Severity::Warning, "after clear"), Ok(3));
        let output = frame(&ctx, &mut panel, &mut console, Vec::new());
        assert_eq!(visible_records(&output), ["#3 [Warning] after clear"]);
    }
}
