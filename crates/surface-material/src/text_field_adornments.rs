use surface::{Rect, TextMeasurer, TextStyle};

use crate::VectorIcon;

/// The two Material 3 single-line text field appearances.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TextFieldAppearance {
    Outlined,
    Filled,
}

/// Passive text beside the input value, with a localized spoken form.
/// The affix stays separate from the value used for editing and submission.
#[derive(Clone, Copy, Debug)]
pub struct TextFieldAffix<'a> {
    pub visual: &'a str,
    pub spoken_name: &'a str,
}

/// The optional button at the logical end of a field.
#[derive(Clone, Debug)]
pub struct TextFieldTrailingAction<'a, M> {
    pub icon: &'a VectorIcon,
    pub name: &'a str,
    pub disabled: bool,
    pub on_press: M,
}

/// Optional content for either Material 3 single-line field appearance.
#[derive(Clone, Debug)]
pub struct TextFieldAdornments<'a, M> {
    pub leading_icon: Option<&'a VectorIcon>,
    pub trailing_action: Option<TextFieldTrailingAction<'a, M>>,
    pub prefix: Option<TextFieldAffix<'a>>,
    pub suffix: Option<TextFieldAffix<'a>>,
}

impl<M> Default for TextFieldAdornments<'_, M> {
    fn default() -> Self {
        Self {
            leading_icon: None,
            trailing_action: None,
            prefix: None,
            suffix: None,
        }
    }
}

pub(super) const ICON_SIZE: f32 = 24.;
pub(super) const ACTION_SIZE: f32 = 48.;
pub(super) const SIDE: f32 = 16.;
const ICON_INSET: f32 = 52.;
const AFFIX_GAP: f32 = 2.;
const MIN_VALUE_WIDTH: f32 = 48.;

#[derive(Debug)]
pub(super) struct AffixText {
    pub visual: String,
    pub spoken_name: String,
}

#[derive(Debug, Default)]
pub(super) struct FieldAdornments {
    pub leading: bool,
    pub trailing: bool,
    pub prefix: Option<AffixText>,
    pub suffix: Option<AffixText>,
}

pub(super) struct Slots {
    pub leading: bool,
    pub trailing_inline: bool,
    pub body_start: f32,
    pub body_end: f32,
    pub prefix_width: f32,
    pub suffix_width: f32,
    pub affixes_inline: bool,
}

impl FieldAdornments {
    pub(super) fn has_slots(&self) -> bool {
        self.leading || self.trailing || self.prefix.is_some() || self.suffix.is_some()
    }

    pub(super) fn measure(
        &self,
        width: f32,
        fonts: &mut dyn TextMeasurer,
        body: &TextStyle,
        line_height: f32,
    ) -> Slots {
        let single_line = |affix: &AffixText, fonts: &mut dyn TextMeasurer| {
            let (width, height) = fonts.measure_text(&affix.visual, body, None);
            (width.is_finite() && width > 0. && height <= line_height + 0.5).then_some(width)
        };
        let prefix = self
            .prefix
            .as_ref()
            .map_or(Some(0.), |affix| single_line(affix, fonts));
        let suffix = self
            .suffix
            .as_ref()
            .map_or(Some(0.), |affix| single_line(affix, fonts));
        let mut leading = self.leading;
        let mut trailing_inline = self.trailing;
        let mut body_start = if leading { ICON_INSET } else { SIDE };
        let mut body_end = width - if trailing_inline { ICON_INSET } else { SIDE };
        let affixes_inline = prefix.zip(suffix).is_some_and(|(prefix, suffix)| {
            let gaps =
                if prefix > 0. { AFFIX_GAP } else { 0. } + if suffix > 0. { AFFIX_GAP } else { 0. };
            body_end - body_start - prefix - suffix - gaps >= MIN_VALUE_WIDTH
        });
        if leading && body_end - body_start < MIN_VALUE_WIDTH {
            leading = false;
            body_start = SIDE;
        }
        if trailing_inline && body_end - body_start < MIN_VALUE_WIDTH {
            trailing_inline = false;
            body_end = width - SIDE;
        }
        Slots {
            leading,
            trailing_inline,
            body_start,
            body_end,
            prefix_width: prefix.unwrap_or(0.),
            suffix_width: suffix.unwrap_or(0.),
            affixes_inline,
        }
    }
}

impl Slots {
    pub(super) fn value(&self, y: f32, height: f32) -> Rect {
        let mut start = self.body_start;
        let mut end = self.body_end;
        if self.affixes_inline {
            if self.prefix_width > 0. {
                start += self.prefix_width + AFFIX_GAP;
            }
            if self.suffix_width > 0. {
                end -= self.suffix_width + AFFIX_GAP;
            }
        }
        Rect::new(start, y, end - start, height)
    }
}

pub(super) fn physical(rect: Rect, width: f32, rtl: bool) -> Rect {
    if rtl {
        Rect::new(width - rect.x - rect.width, rect.y, rect.width, rect.height)
    } else {
        rect
    }
}
