use super::{DockId, SplitId, TabId};
use crate::appearance::AppearanceProfile;
use crate::tab_titles::TabTitlePresentation;
use eframe::egui::{Id, Rect};

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
        serde_json::json!({
            "profile": self.profile.name(),
            "viewport": rect_json(self.viewport),
            "pixels_per_point": self.pixels_per_point,
            "disabled_transport_reasons": if self.profile == AppearanceProfile::Reference {
                serde_json::json!({
                    "command.play": "Play execution is unsupported.",
                    "command.pause": "Pause requires unsupported Play execution.",
                    "command.step": "Step requires unsupported Play execution."
                })
            } else {
                serde_json::json!({})
            },
            "toolbar": rect_json(self.toolbar),
            "status": rect_json(self.status),
            "body": rect_json(self.body),
            "fractions": self.fractions,
            "active_scene": self.active_scene.label(),
            "active_bottom": self.active_bottom.label(),
            "docks": self.docks
                .iter()
                .map(|d|
                    serde_json::json!({
                        "id": format!("{:?}", d.id),
                        "rect": rect_json(d.rect),
                        "header": rect_json(d.header),
                        "content": rect_json(d.content),
                        "active": d.active.label()
                    })
                )
                .collect::<Vec<_>>(),
            "splitters": self.splitters
                .iter()
                .map(|s|
                    serde_json::json!({
                        "id": format!("{:?}", s.id),
                        "divider": rect_json(s.divider),
                        "hit_rect": rect_json(s.hit_rect)
                    })
                )
                .collect::<Vec<_>>(),
            "tab_titles": self.tab_titles
                .iter()
                .map(TabTitleSnapshot::to_json)
                .collect::<Vec<_>>(),
            "controls": self.controls
                .iter()
                .map(|c|
                    serde_json::json!({
                        "role": c.role,
                        "rect": rect_json(c.rect),
                        "widget_id": c.widget_id.value(),
                        "enabled": c.enabled,
                        "selected": c.selected
                    })
                )
                .collect::<Vec<_>>()
        })
    }
}
