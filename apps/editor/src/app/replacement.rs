use super::*;

impl EditorApp {
    pub(super) fn check_idle(&self) -> Result<(), String> {
        if self.pending.is_some() {
            Err("Resolve the pending unsaved-work decision first.".into())
        } else {
            Ok(())
        }
    }

    pub(super) fn request(&mut self, action: Pending) -> Result<(), String> {
        self.check_idle()?;
        if self.is_dirty() {
            self.pending = Some(action);
            Ok(())
        } else {
            self.perform(action)
        }
    }

    /// Start a new document, guarded against discarding unsaved work.
    pub fn request_new(&mut self) -> Result<(), String> {
        self.request(Pending::New)
    }

    /// Validate a replacement before prompting to discard the current document.
    pub fn request_open(&mut self, path: &Path) -> Result<(), String> {
        self.check_idle()?;
        let candidate = self.load_document(path)?;
        self.protect_candidate(&candidate);
        self.request(candidate)
    }

    /// Request closing through the same unsaved document guard.
    pub fn request_close(&mut self) -> Result<(), String> {
        self.request(Pending::Close)
    }

    /// Resolve an unsaved guard. Save failure preserves both work and the guard.
    pub fn resolve_unsaved(&mut self, choice: UnsavedChoice) -> Result<(), String> {
        if self.pending.is_none() {
            return Ok(());
        }
        if matches!(choice, UnsavedChoice::Cancel) {
            self.pending = None;
            return Ok(());
        }
        if matches!(choice, UnsavedChoice::Save) {
            self.save_current()?;
        }
        if let Some(action) = self.pending.take() {
            self.perform(action)?;
        }
        Ok(())
    }

    pub(super) fn perform(&mut self, action: Pending) -> Result<(), String> {
        let replacing = !matches!(action, Pending::Close);
        let next_scope = if replacing {
            self.inspector_scope
                .checked_add(1)
                .ok_or("Document identity space exhausted.")?
        } else {
            self.inspector_scope
        };
        match action {
            Pending::New => {
                self.imported = None;
                self.document = Document::new(
                    Scene::new("Untitled").map_err(|e| e.to_string())?,
                    self.preferences.history_capacity,
                );
                self.save_path = None;
                self.scene_text.clear();
            }
            Pending::Open(scene, path) => {
                self.imported = None;
                self.document.replace(scene);
                self.scene_text = path.display().to_string();
                self.save_path = Some(path);
            }
            Pending::Imported(document, path) => {
                self.imported = Some(*document);
                self.scene_text = path
                    .as_ref()
                    .map_or_else(String::new, |p| p.display().to_string());
                self.save_path = path;
            }
            Pending::Close => self.close_allowed = true,
        }
        if replacing {
            self.hierarchy_browser.reset_document();
            self.hierarchy_expansion.clear();
            self.inspector_scope = next_scope;
            self.reconcile_inspector();
        }
        self.inspector_panel = InspectorPanel::default();
        self.imported_inspector = imported::InspectorPanel::default();
        self.imported_viewport = imported::ViewportPanel::default();
        Ok(())
    }

    pub(super) fn unsaved_dialog(&mut self, ctx: &Context) {
        if self.pending.is_none() {
            return;
        }
        let mut choice = None;
        egui::Modal::new(egui::Id::new("unsaved_work")).show(ctx, |ui| {
            ui.heading("Save unsaved work?");
            ui.label("Save successfully before continuing, discard changes, or cancel.");
            if self.save_path.is_none() {
                ui.label(if self.imported.is_some() {
                    "New independent .ouimport save path"
                } else {
                    "New independent .ouscene save path"
                });
                ui.text_edit_singleline(&mut self.scene_text);
            }
            ui.label(&self.status);
            ui.horizontal(|ui| {
                if ui.button("Save").clicked() {
                    choice = Some(UnsavedChoice::Save);
                }
                if ui.button("Discard").clicked() {
                    choice = Some(UnsavedChoice::Discard);
                }
                if ui.button("Cancel").clicked() {
                    choice = Some(UnsavedChoice::Cancel);
                }
            });
        });
        // Escape always cancels replacement; it can never discard or save.
        if ctx.input(|input| input.key_pressed(egui::Key::Escape)) {
            choice = Some(UnsavedChoice::Cancel);
        }
        if let Some(choice) = choice {
            let result = self.resolve_unsaved(choice);
            self.report(result, "Unsaved-work decision applied.");
        }
    }
}
