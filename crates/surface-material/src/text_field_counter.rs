use surface::{Rect, TextMeasurer, TextStyle};

use crate::text_field_adornments::physical;

const ROW_GAP: f32 = 4.;
const COLUMN_GAP: f32 = 16.;
const MIN_HELP_WIDTH: f32 = 48.;

/// A character counter with a soft limit for a single-line Material field.
///
/// The count measures extended grapheme clusters in the application-owned
/// value. It excludes preedit and field affixes. The field keeps text over the
/// limit and uses the localized error to mark it invalid. The caller owns
/// submission policy.
#[derive(Clone, Copy)]
pub struct TextFieldCounterOptions<'a> {
    pub max_graphemes: usize,
    /// A stable, nonempty localized message shown when the value is too long.
    pub over_limit_error: &'a str,
    /// Provide a nonempty description of the count and limit in the user's language.
    pub describe: &'a dyn Fn(usize, usize) -> String,
}

pub(super) fn supporting_row(
    fonts: &mut dyn TextMeasurer,
    style: &TextStyle,
    (description, count): (Option<&str>, Option<&str>),
    bounds: Rect,
    rtl: bool,
) -> (Option<Rect>, Option<Rect>) {
    let line_height = bounds.height;
    let measured_height = |fonts: &mut dyn TextMeasurer, text: &str, width: f32| {
        fonts
            .measure_text(text, style, Some(width))
            .1
            .max(line_height)
            .ceil()
    };
    let width = bounds.width;
    let Some(count) = count else {
        return (
            description.map(|text| {
                Rect::new(
                    bounds.x,
                    bounds.y,
                    width,
                    measured_height(fonts, text, width),
                )
            }),
            None,
        );
    };

    let (natural_count, _) = fonts.measure_text(count, style, None);
    let count_width = natural_count.ceil().clamp(1., width);
    let remaining = width - count_width - COLUMN_GAP;
    let fits_row = description.is_some() && remaining >= MIN_HELP_WIDTH;

    let help_width = if fits_row { remaining } else { width };
    let help = description.map(|text| {
        let height = measured_height(fonts, text, help_width);
        let local = physical(Rect::new(0., 0., help_width, height), width, rtl);
        Rect::new(bounds.x + local.x, bounds.y, local.width, local.height)
    });
    let count_y = match help {
        Some(help) if !fits_row => help.y + help.height + ROW_GAP,
        _ => bounds.y,
    };
    let count_height = measured_height(fonts, count, count_width);
    let local = physical(
        Rect::new(width - count_width, 0., count_width, count_height),
        width,
        rtl,
    );
    let count = Rect::new(bounds.x + local.x, count_y, local.width, local.height);
    (help, Some(count))
}
