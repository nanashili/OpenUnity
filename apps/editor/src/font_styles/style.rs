//! Versioned style identity and exact approved face descriptors.

use eframe::egui::FontFamily;

pub(super) const SEMIBOLD: &[u8] =
    local_reference_bytes!("../../assets/unity-7000.0.0a8/fonts/Inter-SemiBold.ttf");
pub(super) const ITALIC: &[u8] =
    local_reference_bytes!("../../assets/unity-7000.0.0a8/fonts/Inter-Italic.ttf");
pub(super) const SEMIBOLD_ITALIC: &[u8] =
    local_reference_bytes!("../../assets/unity-7000.0.0a8/fonts/Inter-SemiBoldItalic.ttf");

/// Version1 explicit product requests; none implies a Unity control assignment.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProportionalStyleV1 {
    /// The exact reviewed Inter-SemiBold face, whose recorded OS/2 weight is700.
    Semibold,
    /// The exact reviewed Inter-Italic face, without synthetic skew.
    Italic,
    /// The exact reviewed Inter-SemiBoldItalic face, without synthetic transforms.
    SemiboldItalic,
}

impl ProportionalStyleV1 {
    pub(super) fn index(self) -> usize {
        match self {
            Self::Semibold => 0,
            Self::Italic => 1,
            Self::SemiboldItalic => 2,
        }
    }
    pub(super) fn name(self) -> &'static str {
        match self {
            Self::Semibold => "semibold",
            Self::Italic => "italic",
            Self::SemiboldItalic => "semibold_italic",
        }
    }
    pub(super) fn face(self) -> (&'static str, &'static [u8], usize, &'static str) {
        match self {
            Self::Semibold => (
                "Inter-SemiBold",
                SEMIBOLD,
                447_532,
                "f20a40a8443df2e20a547d31187fb747d32263854a19496e41d88f4204082caa",
            ),
            Self::Italic => (
                "Inter-Italic",
                ITALIC,
                439_620,
                "c630b1bb55a96032d0dae52cb6b4e1a9bef8c179398a5df510eb1c767f6c3500",
            ),
            Self::SemiboldItalic => (
                "Inter-SemiBoldItalic",
                SEMIBOLD_ITALIC,
                453_204,
                "a4ca1a33406802e20bac217b0e5ccb25037e68f396392cf7b8cafb64c0933d74",
            ),
        }
    }
    pub(super) fn family(self) -> FontFamily {
        FontFamily::Name(format!("OpenUnity.InterStyle.v1.{}", self.name()).into())
    }
}
