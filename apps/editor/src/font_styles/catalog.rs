//! Validate the bounded embedded catalog without selecting a style.

use std::sync::{Arc, OnceLock};

use eframe::egui::{self, FontData, FontDefinitions};

use super::style::{ITALIC, SEMIBOLD, SEMIBOLD_ITALIC};
use super::{FontStyleCatalog, ProportionalStyleV1};

impl FontStyleCatalog {
    /// Validate the reviewed embedded catalog once, without registering or selecting it.
    pub fn embedded() -> Self {
        static CATALOG: OnceLock<FontStyleCatalog> = OnceLock::new();
        CATALOG
            .get_or_init(|| {
                if cfg!(feature = "local-reference-assets") {
                    Self::from_faces(Some(SEMIBOLD), Some(ITALIC), Some(SEMIBOLD_ITALIC))
                } else {
                    Self::from_faces(None, None, None)
                }
            })
            .clone()
    }
    /// Resolve supplied optional face resources against the exact reviewed originals.
    /// Missing, oversized, foreign or unparseable data remains an honest resource failure.
    pub fn from_faces(
        semibold: Option<&[u8]>,
        italic: Option<&[u8]>,
        semibold_italic: Option<&[u8]>,
    ) -> Self {
        let inputs = [semibold, italic, semibold_italic];
        let styles = [
            ProportionalStyleV1::Semibold,
            ProportionalStyleV1::Italic,
            ProportionalStyleV1::SemiboldItalic,
        ];
        Self {
            faces: std::array::from_fn(|index| {
                let style = styles[index];
                let (name, original, expected, _) = style.face();
                let bytes = inputs[index].ok_or_else(|| {
                    format!(
                        "Reviewed font {name} is unavailable; using the existing regular/fallback family."
                    )
                })?;
                if bytes.len() != expected || bytes.len() > 1_048_576 || bytes != original {
                    return Err(format!(
                        "Reviewed font {name} failed exact resource validation; using the existing regular/fallback family."
                    ));
                }
                let data = Arc::new(FontData::from_static(original));
                let mut probe = FontDefinitions::empty();
                probe.font_data.insert(name.into(), data.clone());
                if std::panic::catch_unwind(|| {
                    egui::epaint::text::Fonts::new(1024, Default::default(), probe)
                })
                .is_err()
                {
                    return Err(format!(
                        "Reviewed font {name} could not be parsed; using the existing regular/fallback family."
                    ));
                }
                Ok(data)
            }),
        }
    }
}
