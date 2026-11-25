use surface::{FillContour, MeasuredGroupLayout, Node, Rect, Role, Style, TextMeasurer};
use surface_app::View;

use super::{Palette, StandardSliderOptions, StandardSliderSize};
use crate::VectorIcon;

/// Decorative icon for a Medium, Large, or ExtraLarge Standard slider.
#[derive(Clone, Copy, Debug)]
pub struct SliderInsetIcon<'a> {
    pub icon: &'a VectorIcon,
    /// Optional replacement at the accepted minimum value.
    pub at_minimum: Option<&'a VectorIcon>,
}

#[derive(Clone, Copy, Debug)]
pub(super) enum Axis {
    Horizontal { reverse: bool },
    Vertical,
}

impl Axis {
    fn axes(self, rect: Rect) -> (f32, f32, f32, f32) {
        match self {
            Self::Horizontal { .. } => (rect.x, rect.width, rect.y, rect.height),
            Self::Vertical => (rect.y, rect.height, rect.x, rect.width),
        }
    }

    fn square(self, major: f32, cross: f32, size: f32) -> Rect {
        match self {
            Self::Horizontal { .. } => Rect::new(major, cross, size, size),
            Self::Vertical => Rect::new(cross, major, size, size),
        }
    }

    fn reversed(self) -> bool {
        match self {
            Self::Horizontal { reverse } => reverse,
            Self::Vertical => true,
        }
    }
}

pub(super) struct IconPaint {
    id: String,
    contours: [FillContour; 2],
    tokens: IconTokens,
}

impl IconPaint {
    pub(super) fn prepare(
        icon: Option<SliderInsetIcon<'_>>,
        options: StandardSliderOptions<'_>,
        palette: &Palette,
        at_minimum: bool,
    ) -> Option<Self> {
        let SliderInsetIcon {
            icon,
            at_minimum: alternate,
        } = icon?;
        let icon = if at_minimum {
            alternate.unwrap_or(icon)
        } else {
            icon
        };
        let (size, padding) = match options.size {
            StandardSliderSize::ExtraSmall | StandardSliderSize::Small => return None,
            StandardSliderSize::Medium | StandardSliderSize::Large => (24., 6.),
            StandardSliderSize::ExtraLarge => (32., 8.),
        };
        // Pair the glyph ink with the track, using the same state colors as
        // the other passive marks on that track.
        Some(Self {
            id: options.id.to_owned(),
            contours: [
                icon.contour_dp(size, palette.active_stop)?,
                icon.contour_dp(size, palette.inactive_stop)?,
            ],
            tokens: IconTokens { size, padding },
        })
    }

    pub(super) fn attach<M>(
        icon: Option<Self>,
        axis: Axis,
        node: Node,
        parts: &mut Vec<View<M>>,
        inner: impl MeasuredGroupLayout + 'static,
    ) -> Node {
        let Some(icon) = icon else {
            return node.measured_group(inner);
        };
        parts.extend(["active", "inactive"].into_iter().zip(icon.contours).map(
            |(side, contour)| {
                // A collapsed slot gives the glyph an empty ancestor clip.
                let id = format!("{}-inset-icon-{side}", icon.id);
                View::new(
                    Node::new(Role::Group, "")
                        .id(id.as_str())
                        .styled(Style::default().clip(true))
                        .measured_group(GlyphClip(icon.tokens.size)),
                )
                .children([View::new(
                    Node::new(Role::Group, "")
                        .id(format!("{id}-glyph"))
                        .fill_contour(contour),
                )])
                .without_interaction(None)
            },
        ));
        node.measured_group(IconLayout {
            axis,
            inner,
            tokens: icon.tokens,
        })
    }
}

#[derive(Clone, Copy, Debug)]
struct IconTokens {
    size: f32,
    padding: f32,
}

#[derive(Debug)]
struct IconLayout<L> {
    axis: Axis,
    inner: L,
    tokens: IconTokens,
}

impl<L: MeasuredGroupLayout> MeasuredGroupLayout for IconLayout<L> {
    fn places_measured_groups(&self) -> bool {
        true
    }

    fn measure(
        &self,
        children: &[&Node],
        available: Option<f32>,
        fonts: &mut dyn TextMeasurer,
    ) -> (f32, f32) {
        self.inner
            .measure(base_children(children), available, fonts)
    }

    fn place(
        &self,
        children: &[&Node],
        size: (f32, f32),
        fonts: &mut dyn TextMeasurer,
    ) -> Vec<Rect> {
        let mut placements = self.inner.place(base_children(children), size, fonts);
        // Standard artwork starts with the rail, active and inactive tracks.
        // The remaining measured rectangles are the existing paint marks.
        let occupied = &placements[3..];
        let reverse = self.axis.reversed();
        let active = self
            .tokens
            .find(self.axis, placements[1], occupied, reverse);
        let inactive = if active.is_none() {
            self.tokens
                .find(self.axis, placements[2], occupied, !reverse)
        } else {
            None
        };
        placements.extend([active.unwrap_or_default(), inactive.unwrap_or_default()]);
        placements
    }
}

fn base_children<'a>(children: &'a [&Node]) -> &'a [&'a Node] {
    &children[..children.len() - 2]
}

#[derive(Debug)]
struct GlyphClip(f32);

impl MeasuredGroupLayout for GlyphClip {
    fn measure(&self, _: &[&Node], _: Option<f32>, _: &mut dyn TextMeasurer) -> (f32, f32) {
        (self.0, self.0)
    }

    fn place(
        &self,
        _: &[&Node],
        (width, height): (f32, f32),
        _: &mut dyn TextMeasurer,
    ) -> Vec<Rect> {
        vec![Rect::new(0., 0., width, height)]
    }
}

impl IconTokens {
    fn find(self, axis: Axis, track: Rect, occupied: &[Rect], from_max: bool) -> Option<Rect> {
        let (edge, length, cross, thickness) = axis.axes(track);
        let required = self.size + 2. * self.padding;
        if length < required || thickness < required {
            return None;
        }
        // For M/L/XL the token padding and centered cross-axis margin
        // keep the entire square inside either rounded track cap.
        let cross_start = cross + (thickness - self.size) / 2.;
        let padded_cross_start = cross_start - self.padding;
        let padded_cross_end = cross_start + self.size + self.padding;
        let oriented = |position: f32, length: f32| {
            if from_max {
                (-position - length, -position)
            } else {
                (position, position + length)
            }
        };
        let (start, end) = oriented(edge, length);
        let mut candidate = start + self.padding;
        let limit = end - self.padding;
        while candidate + self.size <= limit {
            let mut next = candidate;
            for &mark in occupied {
                let (major, width, cross, height) = axis.axes(mark);
                if width <= 0.
                    || height <= 0.
                    || cross >= padded_cross_end
                    || cross + height <= padded_cross_start
                {
                    continue;
                }
                let (mark_start, mark_end) = oriented(major, width);
                if candidate < mark_end + self.padding
                    && candidate + self.size > mark_start - self.padding
                {
                    next = next.max(mark_end + self.padding);
                }
            }
            if next == candidate {
                let major = if from_max {
                    -candidate - self.size
                } else {
                    candidate
                };
                return Some(axis.square(major, cross_start, self.size));
            }
            candidate = next;
        }
        None
    }
}
