use std::rc::Rc;

use super::{
    range, ticks, Color, Corners, CursorStyle, Elevation, LayoutDirection, Length,
    MeasuredGroupLayout, Node, Palette, RangeValue, Rect, Role, SizeTokens, Span,
    StandardSliderMode, StandardSliderOptions, StandardSliderSize, Style, TextMeasurer, Theme,
    Tick, View, FOCUS_OUTSET, FOCUS_STROKE, HANDLE_GAP, INNER_RADIUS, LABEL_BLOCK_PADDING,
    LABEL_GAP, LABEL_INLINE_PADDING, LABEL_MIN_HEIGHT, LABEL_MIN_WIDTH, MIN_TARGET, STOP_INSET,
    STOP_SIZE, TICK_SPACING,
};
use crate::typography;

/// One endpoint of a horizontal Range slider.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RangeSliderThumb {
    Lower,
    Upper,
}

impl RangeSliderThumb {
    fn suffix(self) -> &'static str {
        match self {
            Self::Lower => "lower",
            Self::Upper => "upper",
        }
    }
}

/// A requested change to the controlled interval.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RangeSliderChange {
    pub thumb: RangeSliderThumb,
    pub lower: f64,
    pub upper: f64,
}

/// A horizontal two-thumb Material 3 slider.
#[derive(Clone, Copy, Debug)]
pub struct RangeSliderOptions<'a> {
    /// A unique stable ID for the pair.
    pub id: &'a str,
    /// A localized name for the pair.
    pub name: &'a str,
    /// Localized names that distinguish the two independently focusable ends.
    pub lower_name: &'a str,
    pub upper_name: &'a str,
    pub min: f64,
    pub max: f64,
    pub lower: f64,
    pub upper: f64,
    pub mode: StandardSliderMode,
    pub size: StandardSliderSize,
    pub direction: LayoutDirection,
    pub disabled: bool,
    /// The caller marks the currently focused or held end.
    pub engaged: Option<RangeSliderThumb>,
    /// Localized current values for accessibility and the visible indicator.
    ///
    /// The indicated end needs at least 53 dp of clear space above its
    /// handle. Allow extra space for enlarged or wrapped text.
    pub lower_label: Option<&'a str>,
    pub upper_label: Option<&'a str>,
}

impl Theme {
    /// Build a full-width horizontal Range slider with two named endpoints.
    ///
    /// The caller owns both values. The endpoints can meet. Their focus order
    /// stays Lower then Upper in either layout direction. Each change contains
    /// the current pair and identifies the end that moved.
    ///
    /// Returns `None` for a blank ID, missing or matching thumb names, an
    /// invalid domain or stop lattice, or a reversed interval.
    pub fn range_slider<M>(
        &self,
        options: RangeSliderOptions<'_>,
        on_change: impl Fn(RangeSliderChange) -> M + 'static,
    ) -> Option<View<M>> {
        if [
            options.id,
            options.name,
            options.lower_name,
            options.upper_name,
        ]
        .iter()
        .any(|name| name.trim().is_empty())
            || options.lower_name.trim() == options.upper_name.trim()
            || options.lower > options.upper
        {
            return None;
        }
        let (lower, intervals, input) = range(StandardSliderOptions {
            id: options.id,
            name: options.lower_name,
            min: options.min,
            max: options.max,
            value: options.lower,
            mode: options.mode,
            size: options.size,
            direction: options.direction,
            disabled: options.disabled,
            engaged: false,
            value_label: None,
        })?;
        let upper = RangeValue {
            value: options.upper,
            ..lower
        };
        let upper = RangeValue {
            value: input.resolve_value(upper, upper.value)?,
            ..upper
        };
        let lower_fraction = input.value_fraction(lower)? as f32;
        let upper_fraction = input.value_fraction(upper)? as f32;
        let engaged = options.engaged.filter(|_| !options.disabled);
        let label = match engaged {
            Some(RangeSliderThumb::Lower) => options.lower_label,
            Some(RangeSliderThumb::Upper) => options.upper_label,
            None => None,
        }
        .filter(|text| !text.trim().is_empty());

        let size = options.size.tokens();
        let palette = Palette::new(self, options.disabled);
        let reverse = input.reverse_horizontal;
        let inactive_caps = if reverse {
            [
                (INNER_RADIUS, size.outer_radius),
                (size.outer_radius, INNER_RADIUS),
            ]
        } else {
            [
                (size.outer_radius, INNER_RADIUS),
                (INNER_RADIUS, size.outer_radius),
            ]
        };
        let ticks = ticks(intervals)
            .into_iter()
            .filter(|tick| tick.index != 0)
            .collect::<Vec<_>>();
        let id = |suffix| format!("{}-{suffix}", options.id);
        let mut parts = vec![
            passive(id("rail"), Style::default()),
            passive(
                id("active"),
                track_style(palette.active, (INNER_RADIUS, INNER_RADIUS)),
            ),
            passive(
                id("inactive-min"),
                track_style(palette.inactive, inactive_caps[0]),
            ),
            passive(
                id("inactive-max"),
                track_style(palette.inactive, inactive_caps[1]),
            ),
            passive(id("stop-min"), pill(palette.inactive_stop)),
            passive(id("stop-max"), pill(palette.inactive_stop)),
        ];
        parts.extend(ticks.iter().map(|tick| {
            let between = (lower_fraction..upper_fraction).contains(&tick.fraction);
            passive(
                format!("{}-tick-{}", options.id, tick.index),
                pill(if between {
                    palette.active_stop
                } else {
                    palette.inactive_stop
                }),
            )
        }));

        let handler = Rc::new(on_change);
        let lower_handler = handler.clone();
        let coincident = lower_fraction == upper_fraction;
        let front = engaged.unwrap_or(RangeSliderThumb::Upper);
        let draw_handle = |thumb| !coincident || thumb == front;
        let mut lower_view = thumb(
            self,
            &options,
            RangeSliderThumb::Lower,
            lower,
            palette.active,
            draw_handle(RangeSliderThumb::Lower),
            engaged == Some(RangeSliderThumb::Lower),
        )
        .with_slider_input(input);
        let mut upper_view = thumb(
            self,
            &options,
            RangeSliderThumb::Upper,
            upper,
            palette.active,
            draw_handle(RangeSliderThumb::Upper),
            engaged == Some(RangeSliderThumb::Upper),
        )
        .with_slider_input(input);
        if !options.disabled {
            lower_view = lower_view
                .on_value(move |value| {
                    lower_handler(RangeSliderChange {
                        thumb: RangeSliderThumb::Lower,
                        lower: value,
                        upper: upper.value,
                    })
                })
                .accessible_slider_adjustment()
                .accessibility_scroll_into_view()
                .cursor(CursorStyle::Pointer);
            upper_view = upper_view
                .on_value(move |value| {
                    handler(RangeSliderChange {
                        thumb: RangeSliderThumb::Upper,
                        lower: lower.value,
                        upper: value,
                    })
                })
                .accessible_slider_adjustment()
                .accessibility_scroll_into_view()
                .cursor(CursorStyle::Pointer);
        }
        parts.extend([lower_view, upper_view]);

        let label_thumb = engaged.filter(|_| label.is_some());
        if let (Some(thumb), Some(label)) = (label_thumb, label) {
            let label_id = format!("{}-{}-label", options.id, thumb.suffix());
            let mut label_style = pill(self.colors.inverse_surface);
            label_style.z_index = 2;
            parts.push(passive(label_id.clone(), label_style));
            let mut text_style = Style::default()
                .with_text_style(typography::LABEL_LARGE)
                .foreground(self.colors.inverse_on_surface);
            text_style.z_index = 2;
            parts.push(
                View::new(
                    Node::new(Role::Text, "")
                        .id(format!("{label_id}-text"))
                        .text(label)
                        .styled(text_style),
                )
                .without_interaction(None),
            );
        }

        let height = size.handle_height.max(MIN_TARGET);
        let mut style = Style::row()
            .width(Length::Percent(1.))
            .height(Length::Px(height));
        style.min_width = MIN_TARGET;
        style.min_height = height;
        let view = View::new(
            Node::new(Role::Group, options.name)
                .id(options.id)
                .disabled(options.disabled)
                .styled(style)
                .measured_group(Artwork {
                    size,
                    reverse,
                    fractions: [lower_fraction, upper_fraction],
                    widths: [
                        handle_width(engaged == Some(RangeSliderThumb::Lower)),
                        handle_width(engaged == Some(RangeSliderThumb::Upper)),
                    ],
                    ticks,
                    label_thumb,
                }),
        )
        .children(parts);
        Some(view.with_horizontal_slider_pair(id("rail"), id("lower"), id("upper")))
    }
}

fn handle_width(engaged: bool) -> f32 {
    if engaged {
        2.
    } else {
        4.
    }
}

fn track_style(color: Color, (left, right): (f32, f32)) -> Style {
    Style::default()
        .background(color)
        .corners(Corners::new(left, right, right, left))
}

fn pill(color: Color) -> Style {
    Style::default().background(color).corners(Corners::full())
}

fn passive<M>(id: String, style: Style) -> View<M> {
    View::new(Node::new(Role::Group, "").id(id).styled(style)).without_interaction(None)
}

fn thumb<M>(
    theme: &Theme,
    options: &RangeSliderOptions<'_>,
    thumb: RangeSliderThumb,
    range: RangeValue,
    color: Color,
    draw_handle: bool,
    engaged: bool,
) -> View<M> {
    let id = format!("{}-{}", options.id, thumb.suffix());
    let (name, value_label) = match thumb {
        RangeSliderThumb::Lower => (options.lower_name, options.lower_label),
        RangeSliderThumb::Upper => (options.upper_name, options.upper_label),
    };
    let suffix = |suffix| format!("{id}-{suffix}");
    let shadow = if draw_handle && !options.disabled {
        Elevation::Level1
    } else {
        Elevation::Level0
    };
    // When both handles meet their targets remain separate. Paint only the
    // front one at that position so its width and shadow stay exact.
    let ink = if draw_handle {
        color
    } else {
        Color::TRANSPARENT
    };
    let handle = View::new(
        Node::new(Role::Group, "")
            .id(suffix("handle"))
            .styled(pill(ink))
            .box_shadows(theme.elevation(shadow)),
    )
    .without_interaction(None);
    let focus = View::new(
        Node::new(Role::Group, "")
            .id(suffix("focus"))
            .styled(Style::default().corners(Corners::full()))
            .focus_outline_from_parent()
            .focus_outline_width(FOCUS_STROKE)
            .focus_color(theme.colors.secondary),
    )
    .without_interaction(None);
    // The paint ordering does not change authored lower/upper focus order.
    let style = Style {
        z_index: i32::from(engaged),
        ..Style::default()
    };
    let mut node = Node::new(Role::Slider, name)
        .id(id)
        .range(range)
        .focus_color(Color::TRANSPARENT)
        .styled(style)
        .measured_group(ThumbArtwork {
            width: handle_width(engaged),
            height: options.size.tokens().handle_height,
        });
    if let Some(value) = value_label {
        node = node.with_slider_value_text(value);
    }
    View::new(node).children([handle, focus])
}

#[derive(Debug)]
struct ThumbArtwork {
    width: f32,
    height: f32,
}

impl MeasuredGroupLayout for ThumbArtwork {
    fn measure(&self, _: &[&Node], _: Option<f32>, _: &mut dyn TextMeasurer) -> (f32, f32) {
        (MIN_TARGET, self.height.max(MIN_TARGET))
    }

    fn place(
        &self,
        _: &[&Node],
        (width, height): (f32, f32),
        _: &mut dyn TextMeasurer,
    ) -> Vec<Rect> {
        let x = (width - self.width) / 2.;
        let y = (height - self.height) / 2.;
        vec![
            Rect::new(x, y, self.width, self.height),
            Rect::new(
                x - FOCUS_OUTSET,
                y - FOCUS_OUTSET,
                self.width + 2. * FOCUS_OUTSET,
                self.height + 2. * FOCUS_OUTSET,
            ),
        ]
    }
}

#[derive(Debug)]
struct Artwork {
    size: SizeTokens,
    reverse: bool,
    fractions: [f32; 2],
    widths: [f32; 2],
    ticks: Vec<Tick>,
    label_thumb: Option<RangeSliderThumb>,
}

impl MeasuredGroupLayout for Artwork {
    fn places_measured_groups(&self) -> bool {
        true
    }

    fn measure(&self, _: &[&Node], available: Option<f32>, _: &mut dyn TextMeasurer) -> (f32, f32) {
        (
            available.unwrap_or(MIN_TARGET),
            self.size.handle_height.max(MIN_TARGET),
        )
    }

    fn place(
        &self,
        children: &[&Node],
        (width, height): (f32, f32),
        fonts: &mut dyn TextMeasurer,
    ) -> Vec<Rect> {
        let target_width = width.min(MIN_TARGET);
        let inset = target_width / 2.;
        let rail = width - target_width;
        let x_at = |start: f32, width| {
            inset
                + if self.reverse {
                    rail - start - width
                } else {
                    start
                }
        };
        let centers = self.fractions.map(|fraction| rail * fraction);
        let [lower, upper] = centers;
        let [lower_gap, upper_gap] = self.widths.map(|width| HANDLE_GAP + width / 2.);
        let active = Span::clipped(lower + lower_gap, upper - upper_gap, rail);
        let minimum = Span::clipped(0., lower - lower_gap, rail);
        let maximum = Span::clipped(upper + upper_gap, rail, rail);
        let stop = |start| {
            Rect::new(
                x_at(start, STOP_SIZE),
                (height - STOP_SIZE) / 2.,
                STOP_SIZE,
                STOP_SIZE,
            )
        };
        let track_y = (height - self.size.track_height) / 2.;
        let track = |span: Span| {
            Rect::new(
                x_at(span.start, span.width()),
                track_y,
                span.width(),
                self.size.track_height,
            )
        };
        let (min_stop, max_stop) = (minimum.has_stop(), maximum.has_stop());
        let max_stop_start = rail - STOP_SIZE - STOP_INSET;
        let mut placed = Vec::with_capacity(children.len());
        placed.extend([
            Rect::new(inset, 0., rail, height),
            track(active),
            if min_stop {
                track(minimum)
            } else {
                Rect::default()
            },
            if max_stop {
                track(maximum)
            } else {
                Rect::default()
            },
            if min_stop {
                stop(STOP_INSET)
            } else {
                Rect::default()
            },
            if max_stop {
                stop(max_stop_start)
            } else {
                Rect::default()
            },
        ]);
        let mut previous_tick = f32::NEG_INFINITY;
        for tick in &self.ticks {
            let start = rail * tick.fraction - STOP_SIZE / 2.;
            let fits_minimum =
                min_stop && start >= STOP_INSET + TICK_SPACING && minimum.fits_tick(start);
            let fits_maximum =
                max_stop && start <= max_stop_start - TICK_SPACING && maximum.fits_tick(start);
            let fits = start >= previous_tick + TICK_SPACING
                && (fits_minimum || active.fits_tick(start) || fits_maximum);
            placed.push(if fits { stop(start) } else { Rect::default() });
            if fits {
                previous_tick = start;
            }
        }
        placed.extend(centers.map(|center| {
            Rect::new(
                x_at(center - target_width / 2., target_width),
                0.,
                target_width,
                height,
            )
        }));
        if let Some(thumb) = self.label_thumb {
            let center = match thumb {
                RangeSliderThumb::Lower => lower,
                RangeSliderThumb::Upper => upper,
            };
            let label = children.last().expect("value indicator text child");
            placed.extend(label_rects(
                label,
                x_at(center, 0.),
                (width, height),
                self.size.handle_height,
                fonts,
            ));
        }
        placed
    }
}

fn label_rects(
    label: &Node,
    center: f32,
    (width, height): (f32, f32),
    handle_height: f32,
    fonts: &mut dyn TextMeasurer,
) -> [Rect; 2] {
    let style = label.style().text_style();
    let text = label.text_value().unwrap_or_default();
    let padding = LABEL_INLINE_PADDING.min(width / 4.);
    let natural_width = fonts.measure_text(text, &style, None).0.ceil();
    let label_width = (natural_width + 2. * padding)
        .max(LABEL_MIN_WIDTH)
        .min(width);
    let line_limit = label_width - 2. * padding;
    let (measured_width, measured_height) = fonts.measure_text(text, &style, Some(line_limit));
    let text_width = measured_width.ceil().min(line_limit);
    let text_height = measured_height.max(style.line_height_px());
    let label_height = (text_height + 2. * LABEL_BLOCK_PADDING).max(LABEL_MIN_HEIGHT);
    let x = (center - label_width / 2.).clamp(0., width - label_width);
    let y = (height - handle_height) / 2. - FOCUS_OUTSET - LABEL_GAP - label_height;
    [
        Rect::new(x, y, label_width, label_height),
        Rect::new(
            x + (label_width - text_width) / 2.,
            y + (label_height - text_height) / 2.,
            text_width,
            text_height,
        ),
    ]
}
