//! The Material 3 type scale as renderer-independent text styles.
//!
//! Values come from `TypeScaleTokens.kt` in androidx-main: size, line height,
//! letter spacing and weight. Older published M3 tables differ slightly for
//! Display Large (-0.25), Body Medium (0.25) and Title Medium (0.15). Surface
//! follows androidx-main. Every style uses the default font family. Brand and
//! Plain are not separate families yet, and there is no per-locale scale.
//! Letter spacing is carried in the style but renderers do not apply it yet.
use surface::{FontFamily, TextStyle};

const fn role(size: f32, line_height: f32, letter_spacing: f32, weight: u16) -> TextStyle {
    TextStyle {
        font_size: size,
        line_height: Some(line_height),
        weight,
        letter_spacing,
        family: FontFamily::Default,
    }
}
const REGULAR: u16 = TextStyle::WEIGHT_REGULAR;
const MEDIUM: u16 = TextStyle::WEIGHT_MEDIUM;
const BOLD: u16 = TextStyle::WEIGHT_BOLD;

pub const DISPLAY_LARGE: TextStyle = role(57., 64., -0.2, REGULAR);
pub const DISPLAY_MEDIUM: TextStyle = role(45., 52., 0., REGULAR);
pub const DISPLAY_SMALL: TextStyle = role(36., 44., 0., REGULAR);
pub const HEADLINE_LARGE: TextStyle = role(32., 40., 0., REGULAR);
pub const HEADLINE_MEDIUM: TextStyle = role(28., 36., 0., REGULAR);
pub const HEADLINE_SMALL: TextStyle = role(24., 32., 0., REGULAR);
pub const TITLE_LARGE: TextStyle = role(22., 28., 0., REGULAR);
pub const TITLE_MEDIUM: TextStyle = role(16., 24., 0.2, MEDIUM);
pub const TITLE_SMALL: TextStyle = role(14., 20., 0.1, MEDIUM);
pub const BODY_LARGE: TextStyle = role(16., 24., 0.5, REGULAR);
pub const BODY_MEDIUM: TextStyle = role(14., 20., 0.2, REGULAR);
pub const BODY_SMALL: TextStyle = role(12., 16., 0.4, REGULAR);
pub const LABEL_LARGE: TextStyle = role(14., 20., 0.1, MEDIUM);
pub const LABEL_MEDIUM: TextStyle = role(12., 16., 0.5, MEDIUM);
pub const LABEL_SMALL: TextStyle = role(11., 16., 0.5, MEDIUM);

pub const DISPLAY_LARGE_EMPHASIZED: TextStyle = role(57., 64., 0., MEDIUM);
pub const DISPLAY_MEDIUM_EMPHASIZED: TextStyle = role(45., 52., 0., MEDIUM);
pub const DISPLAY_SMALL_EMPHASIZED: TextStyle = role(36., 44., 0., MEDIUM);
pub const HEADLINE_LARGE_EMPHASIZED: TextStyle = role(32., 40., 0., MEDIUM);
pub const HEADLINE_MEDIUM_EMPHASIZED: TextStyle = role(28., 36., 0., MEDIUM);
pub const HEADLINE_SMALL_EMPHASIZED: TextStyle = role(24., 32., 0., MEDIUM);
pub const TITLE_LARGE_EMPHASIZED: TextStyle = role(22., 28., 0., MEDIUM);
pub const TITLE_MEDIUM_EMPHASIZED: TextStyle = role(16., 24., 0.15, BOLD);
pub const TITLE_SMALL_EMPHASIZED: TextStyle = role(14., 20., 0.1, BOLD);
pub const BODY_LARGE_EMPHASIZED: TextStyle = role(16., 24., 0.15, MEDIUM);
pub const BODY_MEDIUM_EMPHASIZED: TextStyle = role(14., 20., 0.25, MEDIUM);
pub const BODY_SMALL_EMPHASIZED: TextStyle = role(12., 16., 0.4, MEDIUM);
pub const LABEL_LARGE_EMPHASIZED: TextStyle = role(14., 20., 0.1, BOLD);
pub const LABEL_MEDIUM_EMPHASIZED: TextStyle = role(12., 16., 0.5, BOLD);
pub const LABEL_SMALL_EMPHASIZED: TextStyle = role(11., 16., 0.5, BOLD);
