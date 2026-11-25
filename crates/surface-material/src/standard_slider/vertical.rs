use super::{
    range, ticks, Artwork, Palette, FOCUS_OUTSET, FOCUS_STROKE, INNER_RADIUS, LABEL_BLOCK_PADDING,
    LABEL_GAP, LABEL_INLINE_PADDING, LABEL_MIN_HEIGHT, LABEL_MIN_WIDTH, MIN_TARGET, RAIL_INSET,
};
use crate::{shape::LayoutDirection, typography, Elevation, StandardSliderOptions, Theme};
use surface::{Color, Corners, Length, MeasuredGroupLayout, Node, Rect, Role, Style, TextMeasurer};
use surface_app::{CursorStyle, View};

/// Space allocated to a vertical Standard slider and its optional value label.
#[derive(Clone, Copy, Debug)]
pub struct VerticalStandardSliderPlacement {
    /// The full slider target height in dp, including 12 dp padding at
    /// both ends. Must be finite and at least 48 dp.
    pub height: f32,
    /// Extra dp the caller reserves outside the target at logical trailing.
    /// Includes clearance for the focus ring and optional label. Use zero
    /// for no label paint. The label never adds to the hit target or layout.
    pub label_room: f32,
}

impl Theme {
    /// Build a vertical Standard slider with minimum at the bottom and
    /// maximum at the top. Its cross-axis target is the larger of 48 dp
    /// and the chosen handle size. Direction changes the side of the upright
    /// value label but does not reverse the rail or its keys.
    ///
    /// Allow clearance for the focus outline on both sides. An engaged
    /// label paints only if the caller's reserved side room and the slider
    /// height can contain its measured text. Localized value text remains
    /// available to accessibility even when the visual label is omitted.
    /// Returns `None` for an unnamed slider, invalid range or invalid placement.
    pub fn vertical_standard_slider<M>(
        &self,
        options: StandardSliderOptions<'_>,
        placement: VerticalStandardSliderPlacement,
        handler: impl Fn(f64) -> M + 'static,
    ) -> Option<View<M>> {
        if options.id.trim().is_empty()
            || options.name.trim().is_empty()
            || !placement.height.is_finite()
            || placement.height < MIN_TARGET
            || !placement.label_room.is_finite()
            || placement.label_room < 0.
        {
            return None;
        }
        let (range, intervals, input) = range(options)?;
        let size = options.size.tokens();
        let width = size.handle_height.max(MIN_TARGET);
        let engaged = options.engaged && !options.disabled;
        let focus_overhang = ((size.handle_height - width) / 2. + FOCUS_OUTSET).max(0.);
        let label_width = placement.label_room - focus_overhang - LABEL_GAP;
        let label = options
            .value_label
            .filter(|label| engaged && label_width >= LABEL_MIN_WIDTH && !label.trim().is_empty());
        let fraction = input.value_fraction(range)? as f32;
        let ticks = ticks(intervals);
        let palette = Palette::new(self, options.disabled);
        let pill = |color| Style::default().background(color).corners(Corners::full());
        let track = |color, top, bottom| {
            Style::default()
                .background(color)
                .corners(Corners::new(top, top, bottom, bottom))
        };
        let part = |suffix, style| {
            Node::new(Role::Group, "")
                .id(format!("{}-{suffix}", options.id))
                .styled(style)
        };
        let passive = |node| View::new(node).without_interaction(None);
        let handle_shadow = self.elevation(if options.disabled {
            Elevation::Level0
        } else {
            Elevation::Level1
        });
        let mut parts = vec![
            passive(part("rail", Style::default())),
            passive(part(
                "active",
                track(palette.active, INNER_RADIUS, size.outer_radius),
            )),
            passive(part(
                "inactive",
                track(palette.inactive, size.outer_radius, INNER_RADIUS),
            )),
            passive(part("handle", pill(palette.active)).box_shadows(handle_shadow)),
            passive(
                part("focus", Style::default().corners(Corners::full()))
                    .focus_outline_from_parent()
                    .focus_outline_width(FOCUS_STROKE)
                    .focus_color(self.colors.secondary),
            ),
            passive(part("stop", pill(palette.inactive_stop))),
        ];
        for tick in &ticks {
            let ink = if tick.fraction <= fraction {
                palette.active_stop
            } else {
                palette.inactive_stop
            };
            parts.push(passive(
                Node::new(Role::Group, "")
                    .id(format!("{}-tick-{}", options.id, tick.index))
                    .styled(pill(ink)),
            ));
        }
        if let Some(label) = label {
            parts.extend([
                passive(part("label", pill(self.colors.inverse_surface))),
                passive(
                    Node::new(Role::Text, "")
                        .id(format!("{}-label-text", options.id))
                        .text(label)
                        .styled(
                            Style::default()
                                .with_text_style(typography::LABEL_LARGE)
                                .foreground(self.colors.inverse_on_surface),
                        ),
                ),
            ]);
        }

        let mut style = Style::column()
            .width(Length::Px(width))
            .height(Length::Px(placement.height))
            .padding(RAIL_INSET);
        style.min_width = width;
        style.min_height = placement.height;
        let mut node = Node::new(Role::Slider, options.name)
            .id(options.id)
            .range(range)
            .disabled(options.disabled)
            .focus_color(Color::TRANSPARENT)
            .styled(style)
            .measured_group(VerticalArtwork {
                width,
                height: placement.height,
                rail: Artwork {
                    size,
                    reverse: true,
                    fraction,
                    neutral: None,
                    handle_width: if engaged { 2. } else { 4. },
                    ticks,
                    has_label: false,
                },
                label: label.map(|_| (options.direction, label_width)),
            });
        if let Some(text) = options.value_label {
            node = node.with_slider_value_text(text);
        }
        let view = View::new(node)
            .children(parts)
            .with_vertical_slider_input(input.scale);
        if options.disabled {
            return Some(view);
        }
        Some(
            view.on_value(handler)
                .accessible_slider_adjustment()
                .accessibility_scroll_into_view()
                .cursor(CursorStyle::Pointer),
        )
    }
}

#[derive(Debug)]
struct VerticalArtwork {
    width: f32,
    height: f32,
    rail: Artwork,
    label: Option<(LayoutDirection, f32)>,
}

impl MeasuredGroupLayout for VerticalArtwork {
    fn measure(&self, _: &[&Node], _: Option<f32>, _: &mut dyn TextMeasurer) -> (f32, f32) {
        (self.width - 2. * RAIL_INSET, self.height - 2. * RAIL_INSET)
    }

    fn place(
        &self,
        children: &[&Node],
        (width, height): (f32, f32),
        fonts: &mut dyn TextMeasurer,
    ) -> Vec<Rect> {
        // The shared planner's reversed x axis becomes the bottom-to-top y
        // axis. All points and clearances transpose together, including ticks.
        let mut parts = self.rail.place(children, (height, width), fonts);
        for part in &mut parts {
            *part = Rect::new(part.y, part.x, part.height, part.width);
        }
        if let Some((direction, limit)) = self.label {
            // The Standard planner authors the handle after rail and both tracks.
            let thumb = parts[3];
            let label = children
                .last()
                .expect("vertical slider indicator text child");
            parts.extend(label_rects(label, direction, limit, height, thumb, fonts));
        }
        parts
    }
}

fn label_rects(
    label: &Node,
    direction: LayoutDirection,
    limit: f32,
    height: f32,
    thumb: Rect,
    fonts: &mut dyn TextMeasurer,
) -> [Rect; 2] {
    let style = label.style().text_style();
    let value = label.text_value().unwrap_or_default();
    let padding = LABEL_INLINE_PADDING.min(limit / 4.);
    let natural_width = fonts.measure_text(value, &style, None).0.ceil();
    let width = (natural_width + 2. * padding)
        .max(LABEL_MIN_WIDTH)
        .min(limit);
    let line_limit = width - 2. * padding;
    let (measured_width, measured_height) = fonts.measure_text(value, &style, Some(line_limit));
    let text_width = measured_width.ceil().min(line_limit);
    let text_height = measured_height.max(style.line_height_px());
    let label_height = (text_height + 2. * LABEL_BLOCK_PADDING).max(LABEL_MIN_HEIGHT);
    if label_height > height {
        return [Rect::default(); 2];
    }

    let x = if direction == LayoutDirection::RightToLeft {
        thumb.x - FOCUS_OUTSET - LABEL_GAP - width
    } else {
        thumb.x + thumb.width + FOCUS_OUTSET + LABEL_GAP
    };
    let y = (thumb.y + (thumb.height - label_height) / 2.).clamp(0., height - label_height);
    [
        Rect::new(x, y, width, label_height),
        Rect::new(
            x + (width - text_width) / 2.,
            y + (label_height - text_height) / 2.,
            text_width,
            text_height,
        ),
    ]
}
