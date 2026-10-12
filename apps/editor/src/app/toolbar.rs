use super::*;

impl EditorApp {
    pub(super) fn toolbar(&mut self, ctx: &Context) {
        let enabled = self.pending.is_none() && self.path_form.is_none();
        let reference = self.workspace.profile() == AppearanceProfile::Reference;
        let project_name = self
            .project
            .as_ref()
            .and_then(|p| p.root().file_name())
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| "OpenUnity".into());
        let toolbar = egui::TopBottomPanel::top("workspace.toolbar")
            .exact_height(30.0)
            .frame(if reference {
                egui::Frame::NONE.fill(appearance::CHROME)
            } else {
                egui::Frame::NONE
            })
            .show(ctx, |ui| {
                ui.add_enabled_ui(enabled, |ui| {
                    ui.horizontal_centered(|ui| {
                        ui.add_space(5.0);
                        self.file_menu(ui);
                        self.edit_menu(ui);
                        self.view_menu(ui, reference);
                        self.toolbar_identity(ui, reference, &project_name);
                    });
                });
                if reference {
                    self.transport(ui, enabled);
                }
            });
        self.workspace.toolbar_rect(toolbar.response.rect);
        self.status_bar(ctx, reference);
    }

    fn toolbar_identity(&mut self, ui: &mut egui::Ui, reference: bool, project_name: &str) {
        if !reference {
            ui.separator();
            for (role, capability) in [
                ("command.play", Capability::Play),
                ("command.build", Capability::Build),
            ] {
                let response = ui.button(capability.label()).on_hover_text(
                    "Unavailable in this independent authoring editor; click for the capability diagnostic.");
                self.workspace.record(role, &response, false);
                if response.clicked() {
                    self.report(
                        capabilities::attempt(capability).map_err(|e| e.to_string()),
                        "",
                    );
                }
            }
            ui.separator();
            ui.weak("OpenUnity");
        } else {
            ui.add_space(8.0);
            ui.add_sized(
                [160.0, 18.0],
                egui::Label::new(egui::RichText::new(project_name).color(appearance::MUTED))
                    .truncate(),
            )
            .on_hover_text(project_name);
        }
    }

    fn status_bar(&mut self, ctx: &Context, reference: bool) {
        let name = self.imported.as_ref().map_or_else(
            || self.document.scene().name(),
            |d| d.plan().scene_name.as_str(),
        );
        let project = self.project.as_ref().map_or_else(
            || "No project".to_owned(),
            |p| p.root().display().to_string(),
        );
        let identity = format!(
            "{} · {} · {name} · {project}",
            self.status,
            if self.is_dirty() { "Modified" } else { "Saved" }
        );
        let small = ctx.content_rect().width() < 875.0 || ctx.content_rect().height() < 543.0;
        let status = egui::TopBottomPanel::bottom("workspace.status")
            .exact_height(20.0)
            .frame(if reference {
                egui::Frame::NONE.fill(appearance::CHROME)
            } else {
                egui::Frame::NONE
            })
            .show(ctx, |ui| {
                ui.horizontal_centered(|ui| {
                    ui.add_space(5.0);
                    let text = if small {
                        format!("Workspace below 875×543; scroll to reach panes. {identity}")
                    } else {
                        identity.clone()
                    };
                    ui.add(egui::Label::new(text).truncate())
                        .on_hover_text(&identity);
                });
            });
        self.workspace.status_rect(status.response.rect);
    }
}
