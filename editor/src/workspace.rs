//! Independent fixed-tab workspace for INFRA-EDITOR-WORKSPACE-001.

use crate::appearance::{self, AppearanceProfile};
use crate::tab_titles::{TabTitleMetadata, TabTitlePresentation, TabTitlePresenter};
use eframe::egui::{self, Context, Id, PointerButton, Rect, Response, Sense, Ui, Vec2};

/// A stable dock identity in the reviewed ordered workspace.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DockId {
    /// Scene hierarchy at upper left.
    Hierarchy,
    /// Scene/Game group in the upper workspace.
    Scene,
    /// Project/Console group below the workspace.
    Project,
    /// Full-height right Inspector.
    Inspector,
}
/// Stable tab identities; these do not identify scene objects.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TabId {
    /// Actual hierarchy panel.
    Hierarchy,
    /// Actual independent authoring preview.
    Scene,
    /// Explicit unsupported game-rendering diagnostic.
    Game,
    /// Actual read-only project browser.
    Project,
    /// Actual editor event console.
    Console,
    /// Actual active-document Inspector.
    Inspector,
}
impl TabId {
    const ALL: [Self; 6] = [
        Self::Hierarchy,
        Self::Scene,
        Self::Game,
        Self::Project,
        Self::Console,
        Self::Inspector,
    ];
    fn index(self) -> usize {
        match self {
            Self::Hierarchy => 0,
            Self::Scene => 1,
            Self::Game => 2,
            Self::Project => 3,
            Self::Console => 4,
            Self::Inspector => 5,
        }
    }
    /// Readable independently authored tab label.
    pub fn label(self) -> &'static str {
        match self {
            Self::Hierarchy => "Hierarchy",
            Self::Scene => "Scene",
            Self::Game => "Game",
            Self::Project => "Project",
            Self::Console => "Console",
            Self::Inspector => "Inspector",
        }
    }
    /// Stable semantic role used for navigation and layout inspection.
    pub fn role(self) -> &'static str {
        match self {
            Self::Hierarchy => "tab.hierarchy",
            Self::Scene => "tab.scene",
            Self::Game => "tab.game",
            Self::Project => "tab.project",
            Self::Console => "tab.console",
            Self::Inspector => "tab.inspector",
        }
    }
}
/// The three adjacent-pair split boundaries.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SplitId {
    /// Left workspace versus full-height Inspector.
    Left,
    /// Upper workspace versus Project/Console.
    Upper,
    /// Hierarchy versus Scene/Game.
    Hierarchy,
}
/// Actual rendered control geometry, useful for keyboard navigation and inspection.
#[derive(Clone, Debug)]
pub struct ControlSnapshot {
    /// Stable control role.
    pub role: String,
    /// Actual egui response rectangle in logical units.
    pub rect: Rect,
    /// Actual accessible widget identity.
    pub widget_id: Id,
    /// Whether the widget accepted input this frame.
    pub enabled: bool,
    /// Whether the tab/control is selected.
    pub selected: bool,
}
/// One ordered dock's actual frame geometry.
#[derive(Clone, Debug)]
pub struct DockSnapshot {
    /// Stable dock identity.
    pub id: DockId,
    /// Complete dock allocation, excluding divider space.
    pub rect: Rect,
    /// The 24-unit header allocation.
    pub header: Rect,
    /// Selected content allocation below the header.
    pub content: Rect,
    /// Currently displayed tab.
    pub active: TabId,
}
/// One actual divider and its wider input hit region.
#[derive(Clone, Debug)]
pub struct SplitterSnapshot {
    /// Stable boundary identity.
    pub id: SplitId,
    /// Independent one-unit visual divider.
    pub divider: Rect,
    /// Actual ten-unit pointer hit rectangle.
    pub hit_rect: Rect,
}
/// One Reference tab's actual semantic presentation and independent title-only region.
#[derive(Clone, Debug)]
pub struct TabTitleSnapshot {
    /// Existing typed tab identity.
    pub tab_id: TabId,
    /// Complete resolved semantic and measured title state.
    pub presentation: TabTitlePresentation,
    /// Title-only allocation, excluding icon and Inspector actions.
    pub title_rect: Rect,
    /// Original icon allocation.
    pub icon_rect: Rect,
    /// Actual activation response, unchanged by metadata.
    pub widget_rect: Rect,
    /// Actual accessible activation identity.
    pub widget_id: Id,
    /// Selected state after this frame's existing tab activation path.
    pub selected: bool,
    /// Clipped galley allocation returned by the production painter.
    pub painted_text_rect: Option<Rect>,
}
impl TabTitleSnapshot {
    /// Encode actual title state without accepting a serialized tab identity.
    pub fn to_json(&self) -> serde_json::Value {
        let mut value = self.presentation.to_json();
        value["tab_id"] = self.tab_id.label().into();
        value["role"] = self.tab_id.role().into();
        value["title_rect"] = rect_json(self.title_rect);
        value["icon_rect"] = rect_json(self.icon_rect);
        value["widget_rect"] = rect_json(self.widget_rect);
        value["widget_id"] = self.widget_id.value().into();
        value["selected"] = self.selected.into();
        value["painted_text_rect"] = self
            .painted_text_rect
            .map(rect_json)
            .unwrap_or(serde_json::Value::Null);
        value
    }
}
/// Read-only current-frame layout state; no document or native object is stored.
#[derive(Clone, Debug)]
pub struct WorkspaceSnapshot {
    /// Explicit appearance profile used by this shell.
    pub profile: AppearanceProfile,
    /// Actual logical drawable viewport.
    pub viewport: Rect,
    /// Actual egui device scale.
    pub pixels_per_point: f32,
    /// Actual top command-bar allocation.
    pub toolbar: Rect,
    /// Actual bottom status allocation.
    pub status: Rect,
    /// Actual workspace body, with scrolling when below profile minima.
    pub body: Rect,
    /// Left, upper and hierarchy fractions, respectively.
    pub fractions: [f32; 3],
    /// Selected Scene/Game tab.
    pub active_scene: TabId,
    /// Selected Project/Console tab.
    pub active_bottom: TabId,
    /// Ordered dock geometry.
    pub docks: Vec<DockSnapshot>,
    /// Ordered split boundary geometry.
    pub splitters: Vec<SplitterSnapshot>,
    /// Controls rendered in this frame, including open menus and forms.
    pub controls: Vec<ControlSnapshot>,
    /// Current Reference title presentations; Legacy retains its existing rendering.
    pub tab_titles: Vec<TabTitleSnapshot>,
}
fn rect_json(rect: Rect) -> serde_json::Value {
    serde_json::json!([rect.min.x, rect.min.y, rect.max.x, rect.max.y])
}
impl WorkspaceSnapshot {
    /// Encode actual semantic layout geometry for diagnostics or native receipts.
    pub fn to_json(&self) -> serde_json::Value {
        serde_json::json!({"profile":self.profile.name(),"viewport":rect_json(self.viewport),"pixels_per_point":self.pixels_per_point,
            "disabled_transport_reasons":if self.profile == AppearanceProfile::Reference { serde_json::json!({"command.play":"Play execution is unsupported.","command.pause":"Pause requires unsupported Play execution.","command.step":"Step requires unsupported Play execution."}) } else {serde_json::json!({})},
            "toolbar":rect_json(self.toolbar),"status":rect_json(self.status),"body":rect_json(self.body),
            "fractions":self.fractions,"active_scene":self.active_scene.label(),"active_bottom":self.active_bottom.label(),
            "docks":self.docks.iter().map(|d|serde_json::json!({"id":format!("{:?}",d.id),"rect":rect_json(d.rect),"header":rect_json(d.header),"content":rect_json(d.content),"active":d.active.label()})).collect::<Vec<_>>(),
            "splitters":self.splitters.iter().map(|s|serde_json::json!({"id":format!("{:?}",s.id),"divider":rect_json(s.divider),"hit_rect":rect_json(s.hit_rect)})).collect::<Vec<_>>(),
            "tab_titles":self.tab_titles.iter().map(TabTitleSnapshot::to_json).collect::<Vec<_>>(),
            "controls":self.controls.iter().map(|c|serde_json::json!({"role":c.role,"rect":rect_json(c.rect),"widget_id":c.widget_id.value(),"enabled":c.enabled,"selected":c.selected})).collect::<Vec<_>>()})
    }
}

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
    fn allocations(&mut self, body: Rect) -> ([Rect; 4], [SplitterSnapshot; 3]) {
        let left_extent = body.width() - 1.0;
        let left = (left_extent * self.fractions[0]).clamp(404.0, left_extent - 276.0);
        self.fractions[0] = left / left_extent;
        let rows = body.height() - 1.0;
        let upper = (rows * self.fractions[1]).clamp(221.0, rows - 271.0);
        self.fractions[1] = upper / rows;
        let columns = left - 1.0;
        let hierarchy = (columns * self.fractions[2]).clamp(201.0, columns - 202.0);
        self.fractions[2] = hierarchy / columns;
        let x = body.left() + left;
        let y = body.top() + upper;
        let hx = body.left() + hierarchy;
        let h = Rect::from_min_max(body.min, egui::pos2(hx, y));
        let scene = Rect::from_min_max(egui::pos2(hx + 1.0, body.top()), egui::pos2(x, y));
        let project = Rect::from_min_max(
            egui::pos2(body.left(), y + 1.0),
            egui::pos2(x, body.bottom()),
        );
        let inspector = Rect::from_min_max(egui::pos2(x + 1.0, body.top()), body.max);
        let left_div = Rect::from_min_max(
            egui::pos2(x, body.top()),
            egui::pos2(x + 1.0, body.bottom()),
        );
        let upper_div = Rect::from_min_max(egui::pos2(body.left(), y), egui::pos2(x, y + 1.0));
        let hierarchy_div = Rect::from_min_max(egui::pos2(hx, body.top()), egui::pos2(hx + 1.0, y));
        (
            [h, scene, project, inspector],
            [
                SplitterSnapshot {
                    id: SplitId::Left,
                    divider: left_div,
                    hit_rect: left_div.expand2(egui::vec2(4.5, 0.0)),
                },
                SplitterSnapshot {
                    id: SplitId::Upper,
                    divider: upper_div,
                    hit_rect: upper_div.expand2(egui::vec2(0.0, 4.5)),
                },
                SplitterSnapshot {
                    id: SplitId::Hierarchy,
                    divider: hierarchy_div,
                    hit_rect: hierarchy_div.expand2(egui::vec2(4.5, 0.0)),
                },
            ],
        )
    }
    fn draw(
        &mut self,
        ui: &mut Ui,
        body: Rect,
        enabled: bool,
        render: &mut impl FnMut(&mut Ui, TabId, bool),
    ) {
        let (_, splitters) = self.allocations(body);
        for splitter in &splitters {
            let mut response = ui.interact(
                splitter.hit_rect,
                Id::new(("workspace.split", splitter.id as u8)),
                if enabled {
                    Sense::drag()
                } else {
                    Sense::hover()
                },
            );
            response = response.on_hover_cursor(if splitter.id == SplitId::Upper {
                egui::CursorIcon::ResizeVertical
            } else {
                egui::CursorIcon::ResizeHorizontal
            });
            if enabled && response.dragged_by(PointerButton::Primary) {
                let delta = response.drag_delta();
                let (index, extent, movement) = match splitter.id {
                    SplitId::Left => (0, body.width() - 1.0, delta.x),
                    SplitId::Upper => (1, body.height() - 1.0, delta.y),
                    SplitId::Hierarchy => {
                        (2, (body.width() - 1.0) * self.fractions[0] - 1.0, delta.x)
                    }
                };
                if movement.is_finite() {
                    self.fractions[index] += movement / extent;
                }
            }
        }
        let (rects, splitters) = self.allocations(body);
        for splitter in &splitters {
            ui.painter().rect_filled(
                splitter.divider,
                0.0,
                if self.profile == AppearanceProfile::Reference {
                    appearance::DIVIDER
                } else {
                    ui.visuals().widgets.noninteractive.bg_stroke.color
                },
            );
        }
        self.snapshot.body = body;
        self.snapshot.fractions = self.fractions;
        self.snapshot.splitters = splitters.into();
        for (id, rect, tabs) in [
            (DockId::Hierarchy, rects[0], &[TabId::Hierarchy][..]),
            (DockId::Scene, rects[1], &[TabId::Scene, TabId::Game][..]),
            (
                DockId::Project,
                rects[2],
                &[TabId::Project, TabId::Console][..],
            ),
            (DockId::Inspector, rects[3], &[TabId::Inspector][..]),
        ] {
            let header = Rect::from_min_size(rect.min, egui::vec2(rect.width(), 24.0));
            let content = Rect::from_min_max(egui::pos2(rect.left(), header.bottom()), rect.max);
            ui.painter().rect_filled(rect, 0.0, ui.visuals().panel_fill);
            ui.painter()
                .rect_filled(header, 0.0, ui.visuals().faint_bg_color);
            let mut active = match id {
                DockId::Scene => self.scene,
                DockId::Project => self.bottom,
                DockId::Hierarchy => TabId::Hierarchy,
                DockId::Inspector => TabId::Inspector,
            };
            let inspector_actions =
                self.profile == AppearanceProfile::Reference && id == DockId::Inspector;
            let title_header = if inspector_actions {
                Rect::from_min_max(
                    header.min,
                    egui::pos2((header.right() - 48.0).max(header.left()), header.bottom()),
                )
            } else {
                header
            };
            let mut header_ui = ui.new_child(
                egui::UiBuilder::new()
                    .id_salt(("workspace.header", id as u8))
                    .max_rect(title_header.shrink2(egui::vec2(
                        if self.profile == AppearanceProfile::Reference {
                            0.0
                        } else {
                            5.0
                        },
                        0.0,
                    )))
                    .layout(egui::Layout::left_to_right(egui::Align::Center)),
            );
            header_ui.set_clip_rect(title_header.intersect(ui.clip_rect()));
            if !enabled {
                header_ui.disable();
            }
            let first_control = self.snapshot.controls.len();
            if self.profile == AppearanceProfile::Reference {
                header_ui.spacing_mut().item_spacing.x = 0.0;
            }
            for tab in tabs {
                let response = if self.profile == AppearanceProfile::Reference {
                    let selected = active == *tab;
                    let font = egui::FontId::proportional(12.0);
                    let text = header_ui.painter().layout_no_wrap(
                        tab.label().to_owned(),
                        font.clone(),
                        appearance::TEXT,
                    );
                    let width = (text.size().x + 16.0 + 4.0 + 16.0).min(if inspector_actions {
                        title_header.width()
                    } else {
                        f32::INFINITY
                    });
                    let response = header_ui.add_sized(
                        [width, 24.0],
                        egui::Button::new("")
                            .selected(selected)
                            .fill(if selected {
                                appearance::BAR
                            } else {
                                appearance::CHROME
                            })
                            .stroke(egui::Stroke::NONE)
                            .corner_radius(0.0),
                    );
                    let icon_rect = egui::Rect::from_min_size(
                        egui::pos2(response.rect.left() + 8.0, response.rect.center().y - 8.0),
                        egui::vec2(16.0, 16.0),
                    );
                    let icon_role = match tab {
                        TabId::Hierarchy => "tab_hierarchy",
                        TabId::Scene => "tab_scene",
                        TabId::Game => "tab_game",
                        TabId::Project => "tab_project",
                        TabId::Console => "tab_console",
                        TabId::Inspector => "tab_inspector",
                    };
                    crate::resources::paint(
                        &header_ui,
                        icon_role,
                        tab.role(),
                        response.id,
                        icon_rect,
                    );
                    let text_left = icon_rect.right() + 4.0;
                    let title_rect = Rect::from_min_max(
                        egui::pos2(text_left, response.rect.top()),
                        egui::pos2(
                            (response.rect.right() - 8.0).max(text_left),
                            response.rect.bottom(),
                        ),
                    );
                    let presenter = &mut self.title_presenters[tab.index()];
                    let presentation = presenter
                        .update(
                            header_ui.ctx(),
                            &self.titles[tab.index()],
                            title_rect.width(),
                            font,
                        )
                        .clone();
                    response.widget_info(|| {
                        egui::WidgetInfo::selected(
                            egui::WidgetType::SelectableLabel,
                            header_ui.is_enabled(),
                            selected,
                            &presentation.accessible_name,
                        )
                    });
                    let painted_text_rect =
                        presenter.paint(header_ui.painter(), title_rect, appearance::TEXT);
                    let response = if let Some(tooltip) = &presentation.tooltip {
                        response.on_hover_text(tooltip)
                    } else {
                        response
                    };
                    self.snapshot.tab_titles.push(TabTitleSnapshot {
                        tab_id: *tab,
                        presentation,
                        title_rect,
                        icon_rect,
                        widget_rect: response.rect,
                        widget_id: response.id,
                        selected,
                        painted_text_rect,
                    });
                    if selected {
                        header_ui.painter().line_segment(
                            [response.rect.left_top(), response.rect.right_top()],
                            egui::Stroke::new(1.0, appearance::ACCENT),
                        );
                    }
                    response
                } else {
                    header_ui.add_sized(
                        [72.0, 22.0],
                        egui::Button::new(tab.label()).selected(active == *tab),
                    )
                };
                let pointer_down = response.is_pointer_button_down_on()
                    && header_ui.input(|i| i.pointer.button_pressed(PointerButton::Primary));
                if enabled && (pointer_down || response.clicked()) {
                    active = *tab;
                }
                self.record(tab.role(), &response, active == *tab);
            }
            for control in &mut self.snapshot.controls[first_control..] {
                control.selected = control.role == active.role();
            }
            for title in &mut self.snapshot.tab_titles {
                if tabs.contains(&title.tab_id) {
                    title.selected = title.tab_id == active;
                }
            }
            if inspector_actions {
                let action_rect =
                    Rect::from_min_max(egui::pos2(title_header.right(), header.top()), header.max);
                let mut action_ui = ui.new_child(
                    egui::UiBuilder::new()
                        .id_salt("workspace.inspector.actions")
                        .max_rect(action_rect)
                        .layout(egui::Layout::left_to_right(egui::Align::Center)),
                );
                action_ui.set_clip_rect(header.intersect(ui.clip_rect()));
                if !enabled {
                    action_ui.disable();
                }
                render(&mut action_ui, TabId::Inspector, true);
            }
            match id {
                DockId::Scene => self.scene = active,
                DockId::Project => self.bottom = active,
                _ => (),
            }
            let mut pane = ui.new_child(
                egui::UiBuilder::new()
                    .id_salt(("workspace.content", id as u8))
                    .max_rect(content.shrink2(egui::vec2(
                        if self.profile == AppearanceProfile::Reference {
                            4.0
                        } else {
                            6.0
                        },
                        if self.profile == AppearanceProfile::Reference {
                            0.0
                        } else {
                            6.0
                        },
                    )))
                    .layout(egui::Layout::top_down(egui::Align::Min)),
            );
            pane.set_clip_rect(if self.profile == AppearanceProfile::Reference {
                content.intersect(ui.clip_rect())
            } else {
                content.shrink(5.0).intersect(ui.clip_rect())
            });
            if !enabled {
                pane.disable();
            }
            render(&mut pane, active, false);
            self.snapshot.docks.push(DockSnapshot {
                id,
                rect,
                header,
                content,
                active,
            });
        }
        self.snapshot.active_scene = self.scene;
        self.snapshot.active_bottom = self.bottom;
    }
}
