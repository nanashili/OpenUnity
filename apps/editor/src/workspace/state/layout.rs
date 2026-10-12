use super::*;
use crate::appearance;
use crate::workspace::{SplitId, SplitterSnapshot};
use eframe::egui::{Id, PointerButton};

impl Workspace {
    pub(super) fn allocations(&mut self, body: Rect) -> ([Rect; 4], [SplitterSnapshot; 3]) {
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

    pub(super) fn prepare_layout(&mut self, ui: &mut Ui, body: Rect, enabled: bool) -> [Rect; 4] {
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
        rects
    }
}
