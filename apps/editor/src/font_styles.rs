//! Explicit, versioned proportional styles from the reviewed Inter face catalog.
//! No existing editor control is assigned a style by this module.

use std::sync::Arc;

use eframe::egui::FontData;

mod catalog;
mod presenter;
mod registration;
mod selection;
mod style;

pub(crate) use registration::extend_definitions;
pub use selection::FontStyleSelection;
pub use style::ProportionalStyleV1;

/// Bounded exact-face availability used by the explicit style renderer.
/// Candidate data can only resolve to the reviewed embedded originals.
#[derive(Clone)]
pub struct FontStyleCatalog {
    faces: [Result<Arc<FontData>, String>; 3],
}

/// Explicit style request and actual semantic Label renderer, independent of document state.
pub struct FontStylePresenter {
    catalog: FontStyleCatalog,
    selection: Option<FontStyleSelection>,
    diagnostic: Option<String>,
    last_paint: Option<serde_json::Value>,
}
