//! Private independent viewport state and coordinate conversion.

use eframe::egui::{Pos2, Rect};

mod affine;
mod drawing;
mod navigation;
mod view;

/// Independent XY Transform markers and axes, with handle-based picking.
pub struct ViewportPanel {
    center: [f64; 2],
    zoom: f64,
    grid_visible: bool,
}

impl Default for ViewportPanel {
    fn default() -> Self {
        Self {
            center: [0.0; 2],
            zoom: 64.0,
            grid_visible: true,
        }
    }
}

impl ViewportPanel {
    /// Apply the app's authoring grid preference.
    pub fn set_grid_visible(&mut self, visible: bool) {
        self.grid_visible = visible;
    }
    /// Read the grid preference, including toolbar changes.
    pub fn grid_visible(&self) -> bool {
        self.grid_visible
    }

    fn unproject(&self, rect: Rect, point: Pos2) -> [f64; 2] {
        [
            self.center[0] + f64::from(point.x - rect.center().x) / self.zoom,
            self.center[1] - f64::from(point.y - rect.center().y) / self.zoom,
        ]
    }

    fn project(&self, rect: Rect, point: [f64; 3]) -> Option<Pos2> {
        let x = f64::from(rect.center().x) + (point[0] - self.center[0]) * self.zoom;
        let y = f64::from(rect.center().y) - (point[1] - self.center[1]) * self.zoom;
        (x.is_finite() && y.is_finite() && x.abs() < 1.0e7 && y.abs() < 1.0e7)
            .then_some(Pos2::new(x as f32, y as f32))
    }
}

/// Disposable independent T*R*S affine calculation, using f64 preview precision.
#[derive(Clone, Copy)]
struct Affine([[f64; 4]; 3]);
