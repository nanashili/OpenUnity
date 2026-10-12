use super::{ControlSnapshot, TabId, WorkspaceSnapshot};
use crate::appearance::AppearanceProfile;
use crate::tab_titles::{TabTitleMetadata, TabTitlePresenter};
use eframe::egui::{self, Context, Rect, Response, Sense, Ui, Vec2};

mod docks;
mod header;
mod layout;
mod tabs;

/// App-session fixed tabs and finite split ratios, independent of document state.
pub struct Workspace {
    profile: AppearanceProfile,
    fractions: [f32; 3],
    scene: TabId,
    bottom: TabId,
    snapshot: WorkspaceSnapshot,
    titles: [TabTitleMetadata; 6],
    title_presenters: [TabTitlePresenter; 6],
}
impl Default for Workspace {
    fn default() -> Self {
        let fractions = [921.0 / 1206.0, 394.0 / 665.0, 228.0 / 921.0];
        Self {
            profile: AppearanceProfile::Legacy,
            fractions,
            scene: TabId::Scene,
            bottom: TabId::Project,
            titles: std::array::from_fn(|_| TabTitleMetadata::default()),
            title_presenters: std::array::from_fn(|index| {
                TabTitlePresenter::new(TabId::ALL[index])
            }),
            snapshot: WorkspaceSnapshot {
                profile: AppearanceProfile::Legacy,
                viewport: Rect::ZERO,
                pixels_per_point: 1.0,
                toolbar: Rect::ZERO,
                status: Rect::ZERO,
                body: Rect::ZERO,
                fractions,
                active_scene: TabId::Scene,
                active_bottom: TabId::Project,
                docks: Vec::new(),
                splitters: Vec::new(),
                controls: Vec::new(),
                tab_titles: Vec::new(),
            },
        }
    }
}
impl Workspace {
    /// Atomically replace one existing tab's complete independent session metadata.
    /// Empty explicit titles reject the entire operation, retaining prior presentation.
    pub fn replace_tab_title(
        &mut self,
        tab: TabId,
        metadata: TabTitleMetadata,
    ) -> Result<(), String> {
        let metadata = metadata.validated()?;
        self.titles[tab.index()] = metadata;
        Ok(())
    }
    /// Restore one tab's fixed clean defaults, without altering other tabs or documents.
    pub fn clear_tab_title(&mut self, tab: TabId) {
        self.titles[tab.index()] = TabTitleMetadata::default();
    }
    /// Current complete input record, retained across layout/profile/document changes.
    pub fn tab_title_metadata(&self, tab: TabId) -> &TabTitleMetadata {
        &self.titles[tab.index()]
    }
    /// Reset only workspace ratios and active tabs to the reviewed default profile.
    pub fn reset(&mut self) {
        self.fractions = self.profile.fractions();
        self.scene = if self.profile == AppearanceProfile::Reference {
            TabId::Game
        } else {
            TabId::Scene
        };
        self.bottom = TabId::Project;
    }
    /// Select an explicit shell profile and its initial layout, without touching documents.
    pub fn set_profile(&mut self, profile: AppearanceProfile) {
        if self.profile != profile {
            self.profile = profile;
            self.snapshot.profile = profile;
            self.reset();
        }
    }
    /// Current appearance profile.
    pub fn profile(&self) -> AppearanceProfile {
        self.profile
    }
    /// Immutable geometry and controls from the current rendered frame.
    pub fn snapshot(&self) -> &WorkspaceSnapshot {
        &self.snapshot
    }
    pub(crate) fn begin_frame(&mut self, ctx: &Context) {
        self.snapshot.viewport = ctx.content_rect();
        self.snapshot.profile = self.profile;
        self.snapshot.pixels_per_point = ctx.pixels_per_point();
        self.snapshot.controls.clear();
        self.snapshot.tab_titles.clear();
        self.snapshot.docks.clear();
        self.snapshot.splitters.clear();
    }
    pub(crate) fn record(&mut self, role: impl Into<String>, response: &Response, selected: bool) {
        self.snapshot.controls.push(ControlSnapshot {
            role: role.into(),
            rect: response.rect,
            widget_id: response.id,
            enabled: response.enabled(),
            selected,
        });
    }
    pub(crate) fn toolbar_rect(&mut self, rect: Rect) {
        self.snapshot.toolbar = rect;
    }
    pub(crate) fn status_rect(&mut self, rect: Rect) {
        self.snapshot.status = rect;
    }
    pub(crate) fn show_body(
        &mut self,
        ctx: &Context,
        enabled: bool,
        mut render: impl FnMut(&mut Ui, TabId, bool),
    ) {
        egui::CentralPanel::default()
            .frame(egui::Frame::NONE)
            .show(ctx, |ui| {
                let available = ui.available_size();
                let size = Vec2::new(available.x.max(875.0), available.y.max(493.0));
                egui::ScrollArea::both()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        let (body, _) = ui.allocate_exact_size(size, Sense::hover());
                        self.draw(ui, body, enabled, &mut render);
                    });
            });
    }
}
