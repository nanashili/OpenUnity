//! Independently designed session metadata and measured tab-title presentation.

use crate::workspace::TabId;
use eframe::egui::{Color32, Context, FontId, Painter, Rect};

/// Complete replacement input for one existing tab, independent of documents.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct TabTitleMetadata {
    /// Absent means the original fixed label. An explicit title must be nonempty.
    pub full_title: Option<String>,
    /// Optional additional hover detail; an empty string is normalized to absent.
    pub tooltip_detail: Option<String>,
    /// Explicit visual unsaved indication, with no authoritative document effect.
    pub has_unsaved_changes: bool,
}
impl TabTitleMetadata {
    pub(crate) fn validated(mut self) -> Result<Self, String> {
        if self.full_title.as_ref().is_some_and(String::is_empty) {
            return Err("Invalid tab title metadata".into());
        }
        if self.tooltip_detail.as_ref().is_some_and(String::is_empty) {
            self.tooltip_detail = None;
        }
        Ok(self)
    }
    /// Complete diagnostic input record; no identity is accepted from text.
    pub fn to_json(&self) -> serde_json::Value {
        serde_json::json!({"full_title":self.full_title,"tooltip_detail":self.tooltip_detail,
            "has_unsaved_changes":self.has_unsaved_changes})
    }
}

/// Last resolved presentation, retaining complete semantic text separately from ink.
#[derive(Clone, Debug, PartialEq)]
pub struct TabTitlePresentation {
    /// Effective complete title, including the fixed-label fallback.
    pub full_title: String,
    /// Measured text to paint inside the allotted title span.
    pub display_text: String,
    /// Complete hover text when needed, including any explicit detail.
    pub tooltip: Option<String>,
    /// Full title with an explicit unsaved announcement when requested.
    pub accessible_name: String,
    /// Explicit metadata state, even when no marker fits.
    pub has_unsaved_changes: bool,
    /// Whether the complete title and requested marker did not fit.
    pub elided: bool,
    /// Actual egui layout width of the resolved display text.
    pub display_width: f32,
    /// Last valid logical title span.
    pub title_span: f32,
    /// Last valid logical font size.
    pub font_size: f32,
    /// Actual context scale used for the last valid resolution.
    pub pixels_per_point: f32,
    /// Invalid metrics leave all other presentation fields unchanged.
    pub diagnostic: Option<&'static str>,
}
impl TabTitlePresentation {
    /// Encode semantic state and actual measured values for diagnostics.
    pub fn to_json(&self) -> serde_json::Value {
        serde_json::json!({"full_title":self.full_title,"display_text":self.display_text,
            "tooltip":self.tooltip,"accessible_name":self.accessible_name,
            "has_unsaved_changes":self.has_unsaved_changes,"elided":self.elided,
            "display_width":self.display_width,"title_span":self.title_span,
            "font_size":self.font_size,"pixels_per_point":self.pixels_per_point,
            "diagnostic":self.diagnostic})
    }
}

/// One bounded last-presentation cache for an existing typed tab identity.
/// Resolution uses independent egui text layout, with UTF-8 boundary-safe prefixes.
pub struct TabTitlePresenter {
    tab: TabId,
    font: FontId,
    presentation: TabTitlePresentation,
}
impl TabTitlePresenter {
    /// Start with the fixed clean label; no document state is consulted.
    pub fn new(tab: TabId) -> Self {
        Self {
            tab,
            font: FontId::proportional(12.0),
            presentation: TabTitlePresentation {
                full_title: tab.label().into(),
                display_text: tab.label().into(),
                tooltip: None,
                accessible_name: tab.label().into(),
                has_unsaved_changes: false,
                elided: false,
                display_width: 0.0,
                title_span: 0.0,
                font_size: 12.0,
                pixels_per_point: 1.0,
                diagnostic: None,
            },
        }
    }
    /// Last complete resolution, including any invalid-metrics diagnostic.
    pub fn presentation(&self) -> &TabTitlePresentation {
        &self.presentation
    }
    /// Resolve using the current real context fonts/scale and a logical title span.
    /// Invalid metrics preserve prior presentation, without accepting partial metadata.
    pub fn update(
        &mut self,
        ctx: &Context,
        metadata: &TabTitleMetadata,
        title_span: f32,
        font: FontId,
    ) -> &TabTitlePresentation {
        let pixels_per_point = ctx.pixels_per_point();
        if !title_span.is_finite()
            || title_span < 0.0
            || !font.size.is_finite()
            || font.size <= 0.0
            || !pixels_per_point.is_finite()
            || pixels_per_point <= 0.0
        {
            self.presentation.diagnostic = Some("Invalid tab title layout metrics");
            return &self.presentation;
        }
        let full_title = metadata.full_title.as_deref().unwrap_or(self.tab.label());
        let width = |text: &str| {
            ctx.fonts_mut(|fonts| {
                fonts
                    .layout_no_wrap(text.into(), font.clone(), Color32::WHITE)
                    .size()
                    .x
            })
        };
        let full_display = format!(
            "{full_title}{}",
            if metadata.has_unsaved_changes {
                "*"
            } else {
                ""
            }
        );
        let elided = width(&full_display) > title_span;
        let display_text = if !elided {
            full_display
        } else {
            let marker_fits = metadata.has_unsaved_changes && width("*") <= title_span;
            let suffix = if marker_fits { "…*" } else { "…" };
            if width(suffix) <= title_span {
                // Binary search independent, measured UTF-8 prefixes. No character-width
                // estimates, private fitting logic or modification of the complete title.
                let boundaries: Vec<usize> = full_title
                    .char_indices()
                    .map(|(index, _)| index)
                    .chain(std::iter::once(full_title.len()))
                    .collect();
                let (mut low, mut high) = (0, boundaries.len());
                while low + 1 < high {
                    let middle = low + (high - low) / 2;
                    if width(&format!("{}{suffix}", &full_title[..boundaries[middle]]))
                        <= title_span
                    {
                        low = middle;
                    } else {
                        high = middle;
                    }
                }
                format!("{}{suffix}", &full_title[..boundaries[low]])
            } else if marker_fits {
                "*".into()
            } else {
                String::new()
            }
        };
        let detail = metadata
            .tooltip_detail
            .as_deref()
            .filter(|text| !text.is_empty());
        let tooltip = if elided {
            Some(match detail {
                Some(detail) => format!("{full_title}\n{detail}"),
                None => full_title.into(),
            })
        } else {
            detail.map(str::to_owned)
        };
        let accessible_name = if metadata.has_unsaved_changes {
            format!("{full_title} (unsaved changes)")
        } else {
            full_title.into()
        };
        self.presentation = TabTitlePresentation {
            full_title: full_title.into(),
            display_width: width(&display_text),
            display_text,
            tooltip,
            accessible_name,
            has_unsaved_changes: metadata.has_unsaved_changes,
            elided,
            title_span,
            font_size: font.size,
            pixels_per_point,
            diagnostic: None,
        };
        self.font = font;
        &self.presentation
    }
    /// Paint the resolved galley through the actual title-only clip, centered vertically.
    /// Returns the clipped logical galley rectangle; empty/invalid spans paint nothing.
    pub fn paint(&self, painter: &Painter, title_rect: Rect, color: Color32) -> Option<Rect> {
        if self.presentation.display_text.is_empty()
            || !title_rect.is_finite()
            || title_rect.width() <= 0.0
            || title_rect.height() <= 0.0
        {
            return None;
        }
        let painter = painter.with_clip_rect(title_rect.intersect(painter.clip_rect()));
        let galley = painter.layout_no_wrap(
            self.presentation.display_text.clone(),
            self.font.clone(),
            color,
        );
        let position = eframe::egui::pos2(
            title_rect.left(),
            title_rect.center().y - galley.size().y * 0.5,
        );
        let rect = galley
            .rect
            .translate(position.to_vec2())
            .intersect(painter.clip_rect());
        painter.galley(position, galley, color);
        Some(rect)
    }
}
