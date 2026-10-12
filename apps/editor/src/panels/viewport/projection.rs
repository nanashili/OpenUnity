//! XY projection and bounded world-grid/axis drawing.

use eframe::egui::{self, Align2, Color32, FontId, Pos2, Rect, Stroke};

use super::ViewportPanel;

const MAX_GRID_LINES: usize = 128;

impl ViewportPanel {
    pub(super) fn unproject(&self, rect: Rect, point: Pos2) -> [f64; 2] {
        [
            self.center[0] + f64::from(point.x - rect.center().x) / self.zoom,
            self.center[1] - f64::from(point.y - rect.center().y) / self.zoom,
        ]
    }

    pub(super) fn project(&self, rect: Rect, point: [f64; 3]) -> Option<Pos2> {
        let x = f64::from(rect.center().x) + (point[0] - self.center[0]) * self.zoom;
        let y = f64::from(rect.center().y) - (point[1] - self.center[1]) * self.zoom;
        let result = Pos2::new(x as f32, y as f32);
        result.is_finite().then_some(result)
    }

    pub(super) fn draw_grid(&self, painter: &egui::Painter, rect: Rect) {
        if self.grid_visible {
            // Approximately 64 points per grid cell, with a strict line budget.
            let step = 10_f64.powf((64.0 / self.zoom).log10().ceil());
            let low = self.unproject(rect, rect.left_bottom());
            let high = self.unproject(rect, rect.right_top());
            for axis in 0..2 {
                let first = (low[axis] / step).ceil();
                for offset in 0..MAX_GRID_LINES / 2 {
                    let value = (first + offset as f64) * step;
                    if value > high[axis] {
                        break;
                    }
                    let endpoints = if axis == 0 {
                        [[value, low[1], 0.0], [value, high[1], 0.0]]
                    } else {
                        [[low[0], value, 0.0], [high[0], value, 0.0]]
                    };
                    if let (Some(a), Some(b)) = (
                        self.project(rect, endpoints[0]),
                        self.project(rect, endpoints[1]),
                    ) {
                        painter.line_segment([a, b], Stroke::new(1.0, Color32::from_gray(43)));
                    }
                }
            }
        }
        if let Some(origin) = self.project(rect, [0.0; 3]) {
            if rect.y_range().contains(origin.y) {
                painter.line_segment(
                    [
                        Pos2::new(rect.left(), origin.y),
                        Pos2::new(rect.right(), origin.y),
                    ],
                    Stroke::new(1.0, Color32::from_rgb(166, 83, 83)),
                );
                painter.text(
                    Pos2::new(rect.right() - 8.0, origin.y - 4.0),
                    Align2::RIGHT_BOTTOM,
                    "X",
                    FontId::proportional(12.0),
                    Color32::LIGHT_RED,
                );
            }
            if rect.x_range().contains(origin.x) {
                painter.line_segment(
                    [
                        Pos2::new(origin.x, rect.bottom()),
                        Pos2::new(origin.x, rect.top()),
                    ],
                    Stroke::new(1.0, Color32::from_rgb(81, 145, 103)),
                );
                painter.text(
                    Pos2::new(origin.x + 5.0, rect.top() + 6.0),
                    Align2::LEFT_TOP,
                    "Y",
                    FontId::proportional(12.0),
                    Color32::LIGHT_GREEN,
                );
            }
        }
    }
}
