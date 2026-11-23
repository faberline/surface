use surface::TextStyle;

use crate::{
    time_input::{BaselinePeriodLayout, BASELINE_PANEL_PADDING},
    typography, ClockFormat,
};

use super::{
    effective_scale, footer_height, TimeDialLayout, BODY_GAP, FACE, FOOTER_GAP, HORIZONTAL_GAP,
    PANEL_MARGIN, PERIOD_GAP, SEPARATOR,
};

/// Choose a Baseline Dial arrangement that keeps the whole clock face and its
/// controls visible. Use the same viewport and text scale as the builder.
/// Send [crate::DialEvent::SetDialFits] when either changes.
pub fn time_dial_baseline_layout(
    width: f32,
    height: f32,
    text_scale: f32,
    clock: ClockFormat,
) -> Option<TimeDialLayout> {
    if !width.is_finite() || !height.is_finite() {
        return None;
    }
    let order = if width > height {
        [TimeDialLayout::Horizontal, TimeDialLayout::Vertical]
    } else {
        [TimeDialLayout::Vertical, TimeDialLayout::Horizontal]
    };
    order.into_iter().find(|&layout| {
        let metrics = DialMetrics::new(layout, text_scale, clock);
        let body_height = body_height_available(height, text_scale);
        let height_needed = if layout == TimeDialLayout::Vertical {
            metrics.controls_height + BODY_GAP + FACE
        } else {
            metrics.controls_height.max(FACE)
        };
        metrics.panel_width + 2. * PANEL_MARGIN <= width && height_needed <= body_height
    })
}

pub(super) struct DialMetrics {
    pub field_width: f32,
    pub field_height: f32,
    pub separator_width: f32,
    pub controls_width: f32,
    pub panel_width: f32,
    pub period: BaselinePeriodLayout,
    controls_height: f32,
}

impl DialMetrics {
    pub fn new(layout: TimeDialLayout, text_scale: f32, clock: ClockFormat) -> Self {
        let scale = effective_scale(text_scale).max(1.);
        let vertical = layout == TimeDialLayout::Vertical;
        let has_period = clock == ClockFormat::TwelveHour;
        let regular_width = if vertical && !has_period { 114. } else { 96. };
        let number = typography::DISPLAY_LARGE;
        // Grow the reference width continuously toward a full-width pair of
        // digits at 200%. The text renderer also keeps its local fit bound.
        let large_width = 4. * number.font_size + 12.;
        let field_width = regular_width + (large_width - regular_width) * (scale - 1.);
        let field_height = 80_f32.max(number.line_height_px() * scale + 16.);
        let separator_width = SEPARATOR * scale;
        let fields_width = 2. * field_width + separator_width;
        let period = if vertical {
            BaselinePeriodLayout::DialVertical
        } else {
            BaselinePeriodLayout::DialHorizontal
        };
        let (period_width, period_height) = if has_period {
            period.size(scale)
        } else {
            (0., 0.)
        };
        let title = typography::LABEL_MEDIUM.line_height_px() * scale;
        let (controls_width, controls_height) = if vertical {
            let gap = if has_period { PERIOD_GAP } else { 0. };
            (
                fields_width + gap + period_width,
                title + BODY_GAP + field_height.max(period_height),
            )
        } else {
            let below = if has_period {
                BODY_GAP + period_height
            } else {
                0.
            };
            (
                fields_width.max(period_width),
                title + BODY_GAP + field_height + below,
            )
        };
        let body_width = if vertical {
            controls_width.max(FACE)
        } else {
            controls_width + HORIZONTAL_GAP + FACE
        };
        Self {
            field_width,
            field_height,
            separator_width,
            controls_width,
            panel_width: body_width + 2. * BASELINE_PANEL_PADDING,
            period,
            controls_height,
        }
    }
}

pub(super) fn body_height_available(viewport_height: f32, scale: f32) -> f32 {
    viewport_height
        - 2. * (PANEL_MARGIN + BASELINE_PANEL_PADDING)
        - FOOTER_GAP
        - footer_height(effective_scale(scale).max(1.))
}

pub(super) fn number_style(digits: &str, scale: f32, field_width: f32) -> TextStyle {
    let mut style = typography::DISPLAY_LARGE;
    let units = digits
        .chars()
        .map(|digit| if digit.is_ascii() { 0.65 } else { 1. })
        .sum::<f32>()
        .max(1.);
    let size = (style.font_size * scale).min((field_width - 12.) / units);
    let fitted_scale = size / style.font_size;
    style.font_size = size;
    style.line_height = style.line_height.map(|height| height * fitted_scale);
    style
}
