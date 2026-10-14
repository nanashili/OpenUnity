use super::*;

impl EditorApp {
    pub(super) fn smoke_frame(&mut self, ctx: &Context) {
        let Some(receipt) = &self.smoke else {
            return;
        };
        let Ok(mut status) = receipt.lock() else {
            self.close_allowed = true;
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            return;
        };
        status.frames = self.frames;
        status.appearance_profile = Some(self.workspace.profile().name().to_owned());
        status.workspace_snapshot = Some(self.workspace.snapshot().to_json());
        status.appearance_snapshot = Some(self.appearance_snapshot());
        match self.semantic_snapshot() {
            Ok(snapshot) => status.document_snapshot = Some(snapshot),
            Err(error) => status.error = Some(error),
        }
        ctx.input(|input| {
            for event in &input.events {
                if let egui::Event::Screenshot { image, .. } = event
                    && image.size[0] > 0
                    && image.size[1] > 0
                {
                    status.screenshot_size = Some(image.size);
                    status.screenshot_image = Some((**image).clone());
                }
            }
        });
        if self.frames >= 3 && status.screenshot_size.is_some() {
            status.completed = true;
            self.close_allowed = true;
        } else if self.frames >= 600 || self.smoke_started.elapsed() > Duration::from_secs(20) {
            status.error = Some(
                "Native smoke timed out before a nonempty GPU screenshot was delivered.".into(),
            );
            self.close_allowed = true;
        } else if self.frames >= 3 && !self.smoke_requested {
            ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(egui::UserData::default()));
            self.smoke_requested = true;
        }
        ctx.request_repaint_after(Duration::from_millis(16));
    }

    /// Render one shell frame; also used for headless panel acceptance checks.
    pub fn show(&mut self, ctx: &Context) {
        if self.imported.is_none() {
            self.scene_visibility.reconcile(self.document.scene());
        }
        if let Some(error) = appearance::apply(ctx, self.workspace.profile()) {
            self.report(Err(error), "");
        }
        if self.workspace.profile() == AppearanceProfile::Reference {
            let name = self.imported.as_ref().map_or_else(
                || self.document.scene().name(),
                |d| d.plan().scene_name.as_str(),
            );
            let project = self
                .project
                .as_ref()
                .and_then(|p| p.root().file_name())
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_else(|| "No project".into());
            ctx.send_viewport_cmd(egui::ViewportCommand::Title(format!(
                "{name}{} — {project} — OpenUnity",
                if self.is_dirty() { " *" } else { "" }
            )));
        }
        self.frames = self.frames.saturating_add(1);
        if ctx.input(|input| input.viewport().close_requested())
            && !self.close_allowed
            && self.smoke.is_none()
        {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            let result = self.request_close();
            self.report(result, "Close requested.");
        }
        if let Some(error) = crate::resources::begin_frame(ctx, self.workspace.profile()) {
            self.report(Err(error), "");
        }
        self.workspace.begin_frame(ctx);
        self.shortcuts(ctx);
        self.toolbar(ctx);
        self.panels(ctx);
        self.path_dialog(ctx);
        self.unsaved_dialog(ctx);
        self.resource_snapshot = crate::resources::snapshot(ctx);
        self.smoke_frame(ctx);
        if self.close_allowed {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
    }
}
