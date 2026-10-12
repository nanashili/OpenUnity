//! Independently authored compact skin for INFRA-EDITOR-APPEARANCE-001.

use eframe::egui::{self, Color32, Context, FontFamily, FontId, Id, Rect, Response, Stroke, Ui};

/// Explicit shell appearance; it never changes document state.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum AppearanceProfile {
    /// Original bounded workspace and default egui dark skin.
    #[default]
    Legacy,
    /// Screenshot-informed compact, neutral workspace.
    Reference,
}

impl AppearanceProfile {
    /// Stable profile name for command-line options and receipts.
    pub fn name(self) -> &'static str {
        match self {
            Self::Legacy => "legacy",
            Self::Reference => "reference",
        }
    }
    pub(crate) fn fractions(self) -> [f32; 3] {
        match self {
            Self::Legacy => [921.0 / 1206.0, 394.0 / 665.0, 228.0 / 921.0],
            Self::Reference => [3347.0 / 4108.0, 1587.0 / 2276.0, 806.0 / 3347.0],
        }
    }
}

pub(crate) const CHROME: Color32 = Color32::from_rgb(20, 20, 20);
pub(crate) const PANEL: Color32 = Color32::from_rgb(56, 56, 56);
pub(crate) const BAR: Color32 = Color32::from_rgb(60, 60, 60);
pub(crate) const ASSETS: Color32 = Color32::from_rgb(51, 51, 51);
pub(crate) const STRIPE: Color32 = Color32::from_rgb(63, 63, 63);
pub(crate) const TEXT: Color32 = Color32::from_rgb(194, 194, 194);
pub(crate) const MUTED: Color32 = Color32::from_rgb(154, 154, 154);
pub(crate) const DIVIDER: Color32 = Color32::from_rgb(34, 34, 34);
pub(crate) const INPUT: Color32 = Color32::from_rgb(42, 42, 42);
pub(crate) const SELECTION: Color32 = Color32::from_rgb(77, 77, 77);
pub(crate) const ACCENT: Color32 = Color32::from_rgb(61, 136, 199);

// Reviewed logical requests from INFRA-EDITOR-METRICS-001. These are
// independently drawn tree metrics, not measured Unity runtime geometry.
pub(crate) const TREE_BASE_INDENT: f32 = 2.0;
pub(crate) const TREE_DEPTH_INCREMENT: f32 = 14.0;
pub(crate) const TREE_ICON_WIDTH: f32 = 16.0;
pub(crate) const TREE_ICON_LABEL_GAP: f32 = 2.0;
pub(crate) const TREE_LINE_HEIGHT_FALLBACK: f32 = 16.0;

pub(crate) fn tree_indent(depth: usize) -> f32 {
    TREE_BASE_INDENT + depth as f32 * TREE_DEPTH_INCREMENT
}

pub(crate) fn is_reference(ui: &Ui) -> bool {
    ui.ctx()
        .data(|d| d.get_temp::<AppearanceProfile>(Id::new("editor.appearance")))
        == Some(AppearanceProfile::Reference)
}

pub(crate) fn apply(ctx: &Context, profile: AppearanceProfile) -> Option<String> {
    ctx.set_theme(egui::Theme::Dark);
    let previous = ctx.data_mut(|d| {
        let old = d.get_temp::<AppearanceProfile>(Id::new("editor.appearance"));
        d.insert_temp(Id::new("editor.appearance"), profile);
        old
    });
    if previous == Some(profile) {
        return None;
    }
    let mut style = egui::Style {
        visuals: egui::Visuals::dark(),
        ..Default::default()
    };
    if profile == AppearanceProfile::Reference {
        for (kind, size) in [
            (egui::TextStyle::Body, 12.0),
            (egui::TextStyle::Button, 12.0),
            (egui::TextStyle::Small, 11.0),
            (egui::TextStyle::Heading, 13.0),
        ] {
            style
                .text_styles
                .insert(kind, FontId::new(size, FontFamily::Proportional));
        }
        style
            .text_styles
            .insert(egui::TextStyle::Monospace, FontId::monospace(11.0));
        style.spacing.item_spacing = egui::vec2(4.0, 2.0);
        style.spacing.button_padding = egui::vec2(4.0, 1.0);
        style.spacing.interact_size = egui::vec2(32.0, 18.0);
        style.spacing.indent = 12.0;
        style.visuals.override_text_color = Some(TEXT);
        style.visuals.panel_fill = PANEL;
        style.visuals.window_fill = PANEL;
        style.visuals.faint_bg_color = CHROME;
        style.visuals.extreme_bg_color = INPUT;
        style.visuals.selection.bg_fill = SELECTION;
        style.visuals.selection.stroke = Stroke::new(1.0, ACCENT);
        for widget in [
            &mut style.visuals.widgets.noninteractive,
            &mut style.visuals.widgets.inactive,
            &mut style.visuals.widgets.hovered,
            &mut style.visuals.widgets.active,
            &mut style.visuals.widgets.open,
        ] {
            widget.corner_radius = egui::CornerRadius::same(2);
            widget.fg_stroke.color = TEXT;
            widget.bg_stroke = Stroke::new(1.0, DIVIDER);
        }
        style.visuals.widgets.noninteractive.bg_fill = PANEL;
        style.visuals.widgets.noninteractive.weak_bg_fill = PANEL;
        style.visuals.widgets.inactive.bg_fill = INPUT;
        style.visuals.widgets.inactive.weak_bg_fill = BAR;
        style.visuals.widgets.hovered.bg_fill = SELECTION;
        style.visuals.widgets.hovered.weak_bg_fill = SELECTION;
        style.visuals.widgets.active.bg_fill = SELECTION;
        style.visuals.widgets.active.weak_bg_fill = SELECTION;
        style.visuals.window_corner_radius = egui::CornerRadius::same(2);
        style.visuals.menu_corner_radius = egui::CornerRadius::same(2);
    }
    ctx.set_style(style);
    crate::resources::install(ctx, profile)
}

pub(crate) fn rect_json(rect: Rect) -> serde_json::Value {
    serde_json::json!([rect.left(), rect.top(), rect.right(), rect.bottom()])
}

pub(crate) fn control(
    role: impl Into<String>,
    response: &Response,
    selected: bool,
) -> serde_json::Value {
    serde_json::json!({"role":role.into(), "rect":rect_json(response.rect), "widget_id":response.id.value(),
        "enabled":response.enabled(), "selected":selected})
}

/// Small original folder/file glyphs; no reference pixels or assets are used.
pub(crate) fn asset_glyph(ui: &Ui, rect: Rect, folder: bool, color: Color32) {
    let painter = ui.painter();
    if folder {
        let body = Rect::from_min_max(
            egui::pos2(rect.left(), rect.top() + rect.height() * 0.22),
            rect.max,
        );
        painter.rect_filled(body, 2.0, color);
        painter.rect_filled(
            Rect::from_min_size(
                rect.min,
                egui::vec2(rect.width() * 0.43, rect.height() * 0.4),
            ),
            2.0,
            color,
        );
    } else {
        let page = rect.shrink2(egui::vec2(rect.width() * 0.15, 0.0));
        painter.rect_filled(page, 1.0, color);
        for fraction in [0.4, 0.58, 0.76] {
            let y = page.top() + page.height() * fraction;
            painter.line_segment(
                [
                    egui::pos2(page.left() + 3.0, y),
                    egui::pos2(page.right() - 3.0, y),
                ],
                Stroke::new(1.0, ASSETS),
            );
        }
    }
}
