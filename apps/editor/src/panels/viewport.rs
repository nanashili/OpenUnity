//! Independent XY authoring projection of scene primitives, not a game renderer.

mod panel;
mod primitives;
mod projection;
mod transforms;

#[cfg(test)]
mod tests;

/// A labelled orthographic XY authoring view with pointer selection and pan/zoom.
///
/// Primitive vertices use local scale, then X/Y/Z Euler rotations in degrees,
/// then translation, composed through the parent hierarchy. This independent
/// convention does not assert Unity Transform or renderer compatibility.
/// Middle/secondary dragging pans; the wheel zooms around the pointer. Only
/// selection requests leave this panel; scene edits remain owned by the shell.
pub struct ViewportPanel {
    center: [f64; 2],
    zoom: f64,
    grid_visible: bool,
}
