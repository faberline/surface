//! Renderer-neutral text style values shared by layout, paint and renderers.
use serde::{Deserialize, Serialize};

/// A font family request. Renderers choose the face; `Default` is their sans.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FontFamily {
    #[default]
    Default,
    Named(String),
}

/// The complete set of text attributes a renderer needs to shape a run.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TextStyle {
    pub font_size: f32,
    /// Absolute line height in logical pixels. `None` means `font_size * 1.5`.
    pub line_height: Option<f32>,
    /// CSS-style weight from 1 to 1000. Regular is 400.
    pub weight: u16,
    /// Extra advance after each glyph, in logical pixels.
    ///
    /// It is carried in the style. The bundled renderer does not apply it
    /// yet. Measure and paint agree, so layout does not drift.
    pub letter_spacing: f32,
    pub family: FontFamily,
}
impl TextStyle {
    pub const WEIGHT_REGULAR: u16 = 400;
    pub const WEIGHT_MEDIUM: u16 = 500;
    pub const WEIGHT_BOLD: u16 = 700;
    /// The line height used when a style does not set one, as a multiple of size.
    const DEFAULT_LINE_HEIGHT_FACTOR: f32 = 1.5;

    pub const fn new(font_size: f32) -> Self {
        Self {
            font_size,
            line_height: None,
            weight: Self::WEIGHT_REGULAR,
            letter_spacing: 0.,
            family: FontFamily::Default,
        }
    }
    pub fn line_height_px(&self) -> f32 {
        self.line_height
            .unwrap_or(self.font_size * Self::DEFAULT_LINE_HEIGHT_FACTOR)
    }
    /// A legacy style carries only a size. Its paint output predates this module.
    pub fn is_legacy(&self) -> bool {
        self.line_height.is_none()
            && self.weight == Self::WEIGHT_REGULAR
            && self.letter_spacing == 0.
            && self.family == FontFamily::Default
    }
    /// Ratio between this style's line height and the default 1.5 factor that
    /// a size-only measurer reports. `None` when no line height is set.
    pub(crate) fn line_height_scale(&self) -> Option<f32> {
        self.line_height
            .map(|lh| lh / (self.font_size * Self::DEFAULT_LINE_HEIGHT_FACTOR))
    }
}

pub(crate) fn is_regular_weight(weight: &u16) -> bool {
    *weight == TextStyle::WEIGHT_REGULAR
}
pub(crate) fn regular_weight() -> u16 {
    TextStyle::WEIGHT_REGULAR
}
pub(crate) fn is_zero(value: &f32) -> bool {
    *value == 0.
}
pub(crate) fn is_default_family(family: &FontFamily) -> bool {
    matches!(family, FontFamily::Default)
}
