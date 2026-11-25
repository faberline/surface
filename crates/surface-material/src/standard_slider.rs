use std::num::NonZeroU16;

use surface::{
    Color, Corners, Length, MeasuredGroupLayout, Node, RangeValue, Rect, Role, Style, TextMeasurer,
};
use surface_app::{CursorStyle, SliderInputOptions, SliderScale, View};

use crate::{shape::LayoutDirection, typography, Elevation, Theme};

const RAIL_INSET: f32 = 12.;
const MIN_TARGET: f32 = 48.;
const HANDLE_GAP: f32 = 6.;
const INNER_RADIUS: f32 = 2.;
const STOP_SIZE: f32 = 4.;
const STOP_INSET: f32 = 4.;
const TICK_SPACING: f32 = 8.;
const MAX_TICK_CANDIDATES: u16 = 32;
const FOCUS_STROKE: f32 = 3.;
const FOCUS_OUTSET: f32 = 2. + FOCUS_STROKE;
const LABEL_GAP: f32 = 4.;
const LABEL_MIN_WIDTH: f32 = 48.;
const LABEL_MIN_HEIGHT: f32 = 44.;
const LABEL_INLINE_PADDING: f32 = 16.;
const LABEL_BLOCK_PADDING: f32 = 12.;

/// The five horizontal Material 3 track and handle sizes.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum StandardSliderSize {
    #[default]
    ExtraSmall,
    Small,
    Medium,
    Large,
    ExtraLarge,
}

/// How a controlled value is stepped. The endpoints are inclusive.
///
/// A discrete slider rounds the authored value to the nearest stop. Ties
/// round toward the maximum. The stop values must be distinguishable.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum StandardSliderMode {
    Continuous { keyboard_step: f64 },
    Discrete { intervals: u16 },
}

#[derive(Clone, Copy, Debug)]
pub struct StandardSliderOptions<'a> {
    pub id: &'a str,
    /// A localized name for the slider.
    pub name: &'a str,
    pub min: f64,
    pub max: f64,
    pub value: f64,
    pub mode: StandardSliderMode,
    pub size: StandardSliderSize,
    pub direction: LayoutDirection,
    pub disabled: bool,
    /// The owner sets this while the enabled slider is focused or pressed.
    pub engaged: bool,
    /// Optional localized current value for the indicator and accessibility.
    ///
    /// Reserve 53 dp above the handle for the nominal indicator and its
    /// focus outline clearance. Allow more space for enlarged or wrapped text.
    /// The indicator does not change the slider's target or its rail.
    pub value_label: Option<&'a str>,
}

impl Theme {
    /// Build a full-width horizontal Standard slider with one value binding.
    ///
    /// Use a unique stable ID. The caller applies each reported value to its
    /// model. Returns `None` for an unnamed slider or an invalid numeric range.
    pub fn standard_slider<M>(
        &self,
        options: StandardSliderOptions<'_>,
        handler: impl Fn(f64) -> M + 'static,
    ) -> Option<View<M>> {
        if options.id.trim().is_empty() || options.name.trim().is_empty() {
            return None;
        }
        let (range, intervals, input) = range(options)?;
        let size = options.size.tokens();
        let engaged = options.engaged && !options.disabled;
        let label = options
            .value_label
            .filter(|label| engaged && !label.trim().is_empty());
        let reverse = input.reverse_horizontal;
        let fraction = input.value_fraction(range)? as f32;
        let ticks = ticks(intervals);
        let palette = Palette::new(self, options.disabled);
        let handle_width = if engaged { 2. } else { 4. };
        let (leading, trailing) = if reverse {
            (INNER_RADIUS, size.outer_radius)
        } else {
            (size.outer_radius, INNER_RADIUS)
        };
        let track = |color, left, right| {
            Style::default()
                .background(color)
                .corners(Corners::new(left, right, right, left))
        };
        let pill = |color| Style::default().background(color).corners(Corners::full());
        let part = |suffix, style| {
            View::new(
                Node::new(Role::Group, "")
                    .id(format!("{}-{suffix}", options.id))
                    .styled(style),
            )
            .without_interaction(None)
        };

        let handle_shadow = if options.disabled {
            Elevation::Level0
        } else {
            Elevation::Level1
        };
        let handle = View::new(
            Node::new(Role::Group, "")
                .id(format!("{}-handle", options.id))
                .styled(pill(palette.active))
                .box_shadows(self.elevation(handle_shadow)),
        )
        .without_interaction(None);
        let focus = View::new(
            Node::new(Role::Group, "")
                .id(format!("{}-focus", options.id))
                .styled(Style::default().corners(Corners::full()))
                .focus_outline_from_parent()
                .focus_outline_width(FOCUS_STROKE)
                .focus_color(self.colors.secondary),
        )
        .without_interaction(None);
        let mut parts = vec![
            part("rail", Style::default()),
            part("active", track(palette.active, leading, trailing)),
            part("inactive", track(palette.inactive, trailing, leading)),
            handle,
            focus,
            part("stop", pill(palette.inactive_stop)),
        ];
        for tick in &ticks {
            let ink = if tick.fraction <= fraction {
                palette.active_stop
            } else {
                palette.inactive_stop
            };
            parts.push(
                View::new(
                    Node::new(Role::Group, "")
                        .id(format!("{}-tick-{}", options.id, tick.index))
                        .styled(pill(ink)),
                )
                .without_interaction(None),
            );
        }
        if let Some(label) = label {
            parts.extend([
                part("label", pill(self.colors.inverse_surface)),
                View::new(
                    Node::new(Role::Text, "")
                        .id(format!("{}-label-text", options.id))
                        .text(label)
                        .styled(
                            Style::default()
                                .with_text_style(typography::LABEL_LARGE)
                                .foreground(self.colors.inverse_on_surface),
                        ),
                )
                .without_interaction(None),
            ]);
        }

        let height = size.handle_height.max(MIN_TARGET);
        let mut style = Style::row()
            .width(Length::Percent(1.))
            .height(Length::Px(height))
            .padding(RAIL_INSET);
        style.min_width = MIN_TARGET;
        style.min_height = height;
        let mut node = Node::new(Role::Slider, options.name)
            .id(options.id)
            .range(range)
            .disabled(options.disabled)
            .focus_color(Color::TRANSPARENT)
            .styled(style)
            .measured_group(Artwork {
                size,
                reverse,
                fraction,
                handle_width,
                ticks,
                has_label: label.is_some(),
            });
        if let Some(text) = options.value_label {
            node = node.with_slider_value_text(text);
        }
        let view = View::new(node).children(parts);
        if options.disabled {
            return Some(view);
        }
        // Keep the value and hit geometry in one Runtime binding.
        Some(
            view.on_value(handler)
                .with_slider_input(input)
                .accessible_slider_adjustment()
                .accessibility_scroll_into_view()
                .cursor(CursorStyle::Pointer),
        )
    }
}

fn range(
    options: StandardSliderOptions<'_>,
) -> Option<(RangeValue, Option<u16>, SliderInputOptions)> {
    let span = options.max - options.min;
    let (step, intervals) = match options.mode {
        StandardSliderMode::Continuous { keyboard_step } => (keyboard_step, None),
        StandardSliderMode::Discrete { intervals } => (
            span / f64::from(intervals),
            Some(NonZeroU16::new(intervals)?),
        ),
    };
    let mut range = RangeValue {
        min: options.min,
        max: options.max,
        value: options.value,
        step,
    };
    if !range.is_valid() {
        return None;
    }
    let edge_step = step.min(span);
    if range.min + edge_step <= range.min || range.max - edge_step >= range.max {
        return None;
    }
    let input = SliderInputOptions {
        scale: intervals.map_or(SliderScale::Continuous, SliderScale::Discrete),
        reverse_horizontal: options.direction == LayoutDirection::RightToLeft,
    };
    let intervals = intervals.map(NonZeroU16::get);
    if let Some(intervals) = intervals {
        // Most ranges are well above machine precision. Check each stop only
        // when its increment is near the representable spacing at an edge.
        let edge_resolution =
            (range.min.next_up() - range.min).max(range.max - range.max.next_down());
        if step <= edge_resolution {
            let mut previous = range.min;
            for index in 1..=intervals {
                let value = stop_value(range, index, intervals);
                let index_position = f64::from(index) / f64::from(intervals);
                if value <= previous
                    || input.value_fraction(RangeValue { value, ..range }) != Some(index_position)
                {
                    return None;
                }
                previous = value;
            }
        }
    }
    range.value = input.resolve_value(range, range.value)?;
    Some((range, intervals, input))
}

fn stop_value(range: RangeValue, index: u16, intervals: u16) -> f64 {
    if index == 0 {
        range.min
    } else if index == intervals {
        range.max
    } else {
        range.min + (range.max - range.min) * (f64::from(index) / f64::from(intervals))
    }
}

#[derive(Clone, Copy, Debug)]
struct SizeTokens {
    track_height: f32,
    handle_height: f32,
    outer_radius: f32,
}

impl StandardSliderSize {
    fn tokens(self) -> SizeTokens {
        let (track_height, handle_height, outer_radius) = match self {
            Self::ExtraSmall => (16., 44., 8.),
            Self::Small => (24., 44., 8.),
            Self::Medium => (40., 52., 12.),
            Self::Large => (56., 68., 16.),
            Self::ExtraLarge => (96., 108., 28.),
        };
        SizeTokens {
            track_height,
            handle_height,
            outer_radius,
        }
    }
}

struct Palette {
    active: Color,
    inactive: Color,
    active_stop: Color,
    inactive_stop: Color,
}

impl Palette {
    fn new(theme: &Theme, disabled: bool) -> Self {
        let colors = theme.colors;
        if disabled {
            let faint = |Color(r, g, b, _), opacity| Color(r, g, b, opacity);
            Self {
                active: faint(colors.on_surface, 97),
                inactive: faint(colors.on_surface, 31),
                active_stop: faint(colors.inverse_on_surface, 97),
                inactive_stop: faint(colors.on_surface, 97),
            }
        } else {
            Self {
                active: colors.primary,
                inactive: colors.secondary_container,
                active_stop: colors.on_primary,
                inactive_stop: colors.on_secondary_container,
            }
        }
    }
}

#[derive(Debug)]
struct Tick {
    index: u16,
    fraction: f32,
}

fn ticks(intervals: Option<u16>) -> Vec<Tick> {
    let Some(intervals) = intervals else {
        return Vec::new();
    };
    let candidates = u32::from(intervals.min(MAX_TICK_CANDIDATES));
    (0..candidates)
        .map(|candidate| {
            let index = (candidate * u32::from(intervals) / candidates) as u16;
            Tick {
                index,
                fraction: f32::from(index) / f32::from(intervals),
            }
        })
        .collect()
}

#[derive(Debug)]
struct Artwork {
    size: SizeTokens,
    reverse: bool,
    fraction: f32,
    handle_width: f32,
    ticks: Vec<Tick>,
    has_label: bool,
}

impl MeasuredGroupLayout for Artwork {
    fn measure(&self, _: &[&Node], available: Option<f32>, _: &mut dyn TextMeasurer) -> (f32, f32) {
        (
            available.unwrap_or(MIN_TARGET - 2. * RAIL_INSET),
            self.size.handle_height.max(MIN_TARGET) - 2. * RAIL_INSET,
        )
    }

    fn place(
        &self,
        children: &[&Node],
        (width, height): (f32, f32),
        fonts: &mut dyn TextMeasurer,
    ) -> Vec<Rect> {
        let rail = (width - 2. * RAIL_INSET).max(0.);
        let x_at = |start: f32, width| {
            RAIL_INSET
                + if self.reverse {
                    rail - start - width
                } else {
                    start
                }
        };
        let center = rail * self.fraction;
        let half_gap = HANDLE_GAP + self.handle_width / 2.;
        let active_end = (center - half_gap).max(0.);
        let inactive_start = (center + half_gap).min(rail);
        let inactive_width = rail - inactive_start;
        let track_height = self.size.track_height;
        let track_y = (height - track_height) / 2.;
        let handle_y = (height - self.size.handle_height) / 2.;
        let handle = Rect::new(
            x_at(center - self.handle_width / 2., self.handle_width),
            handle_y,
            self.handle_width,
            self.size.handle_height,
        );
        let stop = |start| {
            Rect::new(
                x_at(start, STOP_SIZE),
                (height - STOP_SIZE) / 2.,
                STOP_SIZE,
                STOP_SIZE,
            )
        };
        let max_stop_start = rail - STOP_INSET - STOP_SIZE;
        let show_stop = inactive_width >= STOP_INSET + STOP_SIZE;
        let mut placements = vec![
            Rect::new(RAIL_INSET, 0., rail, height),
            Rect::new(x_at(0., active_end), track_y, active_end, track_height),
            if show_stop {
                Rect::new(
                    x_at(inactive_start, inactive_width),
                    track_y,
                    inactive_width,
                    track_height,
                )
            } else {
                Rect::default()
            },
            handle,
            Rect::new(
                handle.x - FOCUS_OUTSET,
                handle.y - FOCUS_OUTSET,
                handle.width + 2. * FOCUS_OUTSET,
                handle.height + 2. * FOCUS_OUTSET,
            ),
            if show_stop {
                stop(max_stop_start)
            } else {
                Rect::default()
            },
        ];
        let mut previous = f32::NEG_INFINITY;
        for tick in &self.ticks {
            let start = if tick.index == 0 {
                STOP_INSET
            } else {
                rail * tick.fraction - STOP_SIZE / 2.
            };
            let fits_active = start + STOP_SIZE <= active_end;
            let fits_inactive =
                show_stop && start >= inactive_start && start <= max_stop_start - TICK_SPACING;
            if start >= 0. && start >= previous + TICK_SPACING && (fits_active || fits_inactive) {
                placements.push(stop(start));
                previous = start;
            } else {
                placements.push(Rect::default());
            }
        }
        if self.has_label {
            let label = children.last().expect("indicator text child");
            let style = label.style().text_style();
            let value = label.text_value().unwrap_or_default();
            let limit = width.max(1.);
            let padding = LABEL_INLINE_PADDING.min(limit / 4.);
            let natural_width = fonts.measure_text(value, &style, None).0.ceil();
            let label_width = (natural_width + 2. * padding)
                .max(LABEL_MIN_WIDTH)
                .min(limit);
            let line_limit = label_width - 2. * padding;
            let (measured_width, measured_height) =
                fonts.measure_text(value, &style, Some(line_limit));
            let text_width = measured_width.ceil().min(line_limit);
            let text_height = measured_height.max(style.line_height_px());
            let label_height = (text_height + 2. * LABEL_BLOCK_PADDING).max(LABEL_MIN_HEIGHT);
            let thumb_center = handle.x + handle.width / 2.;
            let x = (thumb_center - label_width / 2.).clamp(0., limit - label_width);
            let y = handle.y - FOCUS_OUTSET - LABEL_GAP - label_height;
            placements.extend([
                Rect::new(x, y, label_width, label_height),
                Rect::new(
                    x + (label_width - text_width) / 2.,
                    y + (label_height - text_height) / 2.,
                    text_width,
                    text_height,
                ),
            ]);
        }
        placements
    }
}
