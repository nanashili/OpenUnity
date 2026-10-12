use super::*;

impl EditorApp {
    pub(super) fn open_form(&mut self, kind: PathFormKind) {
        if self.pending.is_some() || self.path_form.is_some() {
            return;
        }
        let primary = match kind {
            PathFormKind::OpenProject => self.project_text.clone(),
            PathFormKind::OpenScene | PathFormKind::Import => String::new(),
            PathFormKind::SaveAs => String::new(),
        };
        self.path_form = Some(PathForm {
            kind,
            primary,
            bindings: self.bindings_text.clone(),
            error: None,
        });
    }

    pub(super) fn path_dialog(&mut self, ctx: &Context) {
        if self.pending.is_some() {
            self.path_form = None;
            return;
        }
        let Some(mut form) = self.path_form.take() else {
            return;
        };
        let (confirm, cancel) = self.path_form_contents(ctx, &mut form);
        if cancel || ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            return;
        }
        if confirm {
            let result = self.confirm_path_form(&form);
            match result {
                Ok(()) => {
                    if matches!(form.kind, PathFormKind::Import) {
                        self.bindings_text = form.bindings;
                    }
                    self.report(Ok(()), "Path command completed.");
                    return;
                }
                Err(error) => {
                    form.error = Some(error.clone());
                    self.report(Err(error), "");
                }
            }
        }
        self.path_form = Some(form);
    }

    fn confirm_path_form(&mut self, form: &PathForm) -> Result<(), String> {
        if form.primary.trim().is_empty() {
            Err("Enter a nonempty path before confirming.".to_owned())
        } else if matches!(form.kind, PathFormKind::Import) && form.bindings.trim().is_empty() {
            Err("Enter an explicit script bindings path.".to_owned())
        } else {
            let path = PathBuf::from(form.primary.trim());

            match form.kind {
                PathFormKind::OpenProject => self.open_project(&path),
                PathFormKind::OpenScene => self.request_open(&path),
                PathFormKind::Import => {
                    self.request_import(&path, &PathBuf::from(form.bindings.trim()))
                }
                PathFormKind::SaveAs => self.save_to(&path, true),
            }
        }
    }

    fn path_form_contents(&mut self, ctx: &Context, form: &mut PathForm) -> (bool, bool) {
        let mut confirm = false;
        let mut cancel = false;
        egui::Modal::new(egui::Id::new("workspace.path_form")).show(ctx, |ui| {
            ui.set_min_width(420.0);
            ui.heading(form.kind.title());
            if matches!(form.kind, PathFormKind::Import) {
                ui.label("Paths are confined to the opened source project. Source files remain read-only.");
            }
            let label = ui.label(match form.kind {
                PathFormKind::OpenProject => "Source project directory",
                PathFormKind::OpenScene => "Independent .ouscene / .ouimport path",
                PathFormKind::Import => "Saved .unity scene path",
                PathFormKind::SaveAs => {
                    if self.imported.is_some() {
                        "New independent .ouimport path"
                    } else {
                        "New independent .ouscene path"
                    }
                }
            });
            let input = ui.text_edit_singleline(&mut form.primary).labelled_by(label.id);
            self.workspace.record("form.primary", &input, false);
            if matches!(form.kind, PathFormKind::Import) {
                let label = ui.label("Explicit script bindings path");
                let input = ui.text_edit_singleline(&mut form.bindings).labelled_by(label.id);
                self.workspace.record("form.bindings", &input, false);
            }
            if let Some(error) = &form.error {
                ui.colored_label(ui.visuals().error_fg_color, error);
            }
            ui.horizontal(|ui| {
                let response = ui.button("Confirm");
                self.workspace.record("form.confirm", &response, false);
                confirm = response.clicked();
                let response = ui.button("Cancel");
                self.workspace.record("form.cancel", &response, false);
                cancel = response.clicked();
            });
        });
        (confirm, cancel)
    }
}

impl PathFormKind {
    fn title(self) -> &'static str {
        match self {
            Self::OpenProject => "Open Project",
            Self::OpenScene => "Open Scene/Archive",
            Self::Import => "Import Saved Scene",
            Self::SaveAs => "Save As",
        }
    }
}
