//! Material carousel keylines and viewport geometry.

use surface::Rect;

use crate::{
    shape::LayoutDirection, CarouselItem, CarouselKind, CarouselOptions, CarouselOrientation,
};

const SIDE_PADDING: f32 = 16.;
const CROSS_PADDING: f32 = 8.;
const GAP: f32 = 8.;
const FULL_SCREEN_GAP: f32 = 16.;
const SMALL_MIN: f32 = 40.;
const SMALL_MAX: f32 = 56.;

/// The visible card and the full-size artwork behind its mask.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct CarouselSlot {
    pub bounds: Rect,
    pub artwork: Rect,
}

#[derive(Clone, Debug)]
struct Keylines {
    edges: Vec<(f32, f32)>,
    outer: f32,
    leading: usize,
    small: f32,
    filler: f32,
    last_edge: f32,
    large: f32,
    large_count: usize,
}

impl Keylines {
    fn new(
        outer: f32,
        available: f32,
        cross: f32,
        preferred: f32,
        count: usize,
        kind: CarouselKind,
    ) -> Self {
        let small = (preferred / 3.).clamp(SMALL_MIN, SMALL_MAX);
        let centered = kind == CarouselKind::HeroCenter;
        let multi = kind == CarouselKind::MultiBrowse;
        let peeks = if centered {
            2 * usize::from(count >= 2 && available >= 3. * SMALL_MIN + 2. * GAP)
        } else {
            usize::from(count >= 2 && available >= 2. * SMALL_MIN + GAP)
        };
        let leading_peek = usize::from(centered && peeks == 2);
        let medium = usize::from(multi && count >= 3 && available >= 3. * SMALL_MIN + 2. * GAP);
        let reserved = small * peeks as f32 + medium as f32 * small / 2.;
        let fixed_gaps = (peeks + medium).saturating_sub(1) as f32 * GAP;
        let large_goal = if multi {
            preferred
        } else {
            available.min(cross * 2.)
        };
        let maximum = count.saturating_sub(peeks + medium).max(1);
        let desired_count = (available - reserved - fixed_gaps - medium as f32 * large_goal / 2.)
            / (large_goal + GAP);
        let large_count = if multi {
            desired_count.round()
        } else {
            desired_count.ceil()
        }
        .clamp(1., maximum as f32) as usize;
        // A centered arrangement needs equal focal slots on both sides of
        // its active item. Use a narrower band if an even count would fit.
        let large_count = if centered && large_count.is_multiple_of(2) {
            large_count - 1
        } else {
            large_count
        };
        let leading = leading_peek + if centered { (large_count - 1) / 2 } else { 0 };
        let separators = (large_count + peeks + medium - 1) as f32 * GAP;
        let allocated = ((available - reserved - separators)
            / (large_count as f32 + medium as f32 / 2.))
            .max(available.min(SMALL_MIN));
        let large = if multi {
            allocated
        } else {
            allocated.min(large_goal)
        };
        let small = small.min(large);
        let middle = (large + small) / 2.;
        let mut widths = Vec::with_capacity(large_count + peeks + medium);
        widths.extend(std::iter::repeat_n(small, leading_peek));
        widths.extend(std::iter::repeat_n(large, large_count));
        widths.extend(std::iter::repeat_n(middle, medium));
        widths.extend(std::iter::repeat_n(small, peeks - leading_peek));
        let total = widths.iter().sum::<f32>() + separators;
        let outer = if centered {
            outer + (available - total).max(0.) / 2.
        } else {
            outer
        };
        let mut cursor = outer;
        let edges = widths
            .into_iter()
            .map(|width| {
                let edge = (cursor, width);
                cursor += width + GAP;
                edge
            })
            .collect();
        Self {
            edges,
            outer,
            leading,
            small,
            // Masked items outside the main keylines continue past the
            // viewport at narrow widths during the same gesture.
            filler: small + GAP,
            last_edge: cursor,
            large,
            large_count,
        }
    }

    fn edge(&self, index: i64) -> (f32, f32) {
        if index < 0 {
            (self.outer + index as f32 * self.filler, self.small)
        } else if let Some(edge) = self.edges.get(index as usize) {
            *edge
        } else {
            (
                self.last_edge + (index - self.edges.len() as i64) as f32 * self.filler,
                self.small,
            )
        }
    }

    fn at(&self, position: f32) -> (f32, f32) {
        let line = position + self.leading as f32;
        let index = line.floor() as i64;
        let fraction = line - index as f32;
        let (x, width) = self.edge(index);
        let (next_x, next_width) = self.edge(index + 1);
        (
            x + (next_x - x) * fraction,
            width + (next_width - width) * fraction,
        )
    }
}

/// Geometry and scroll anchors for an ordered set of carousel items.
///
/// Rebuild this when the viewport, arrangement, aspect ratios or motion
/// preference changes. The same instance drives the controller and view.
#[derive(Clone, Debug)]
pub struct CarouselLayout {
    pub(crate) kind: CarouselKind,
    pub(crate) orientation: CarouselOrientation,
    direction: LayoutDirection,
    viewport: (f32, f32),
    main: f32,
    cross: f32,
    clip: Rect,
    positions: Vec<(f32, f32)>,
    anchors: Vec<f32>,
    max_offset: f32,
    keylines: Option<Keylines>,
}

impl CarouselLayout {
    /// Aspect ratios are width divided by height. The slice also defines
    /// the item count. Only the mixed-aspect arrangement uses the values.
    pub fn new(options: CarouselOptions<'_>, aspect_ratios: &[f32]) -> Self {
        let viewport = (
            finite_extent(options.viewport.0),
            finite_extent(options.viewport.1),
        );
        let horizontal = options.orientation == CarouselOrientation::Horizontal;
        let (main, cross) = if horizontal {
            viewport
        } else {
            (viewport.1, viewport.0)
        };
        let full_screen = options.kind == CarouselKind::FullScreen;
        let outer = if full_screen {
            0.
        } else {
            SIDE_PADDING.min(main / 4.)
        };
        let cross_padding = if full_screen {
            0.
        } else {
            CROSS_PADDING.min(cross / 4.)
        };
        let cross = (cross - 2. * cross_padding).max(0.);
        let contained = options.kind.contained();
        let preferred =
            if options.preferred_item_extent.is_finite() && options.preferred_item_extent > 0. {
                options.preferred_item_extent.min(main)
            } else {
                (main * 0.65).min(240.)
            };
        let valid = main > 0. && cross > 0. && !aspect_ratios.is_empty();
        let layout_keylines = (valid
            && matches!(
                options.kind,
                CarouselKind::MultiBrowse | CarouselKind::HeroStart | CarouselKind::HeroCenter
            ))
        .then(|| {
            Keylines::new(
                outer,
                main - 2. * outer,
                cross,
                preferred,
                aspect_ratios.len(),
                options.kind,
            )
        });
        let band_outer = if contained && !full_screen { outer } else { 0. };
        let (band_start, band_end) = if options.kind == CarouselKind::HeroCenter {
            layout_keylines
                .as_ref()
                .map_or((band_outer, main - band_outer), |lines| {
                    (lines.outer, (lines.last_edge - GAP).min(main - band_outer))
                })
        } else {
            (band_outer, main - band_outer)
        };
        let clip = axis_rect(
            options.orientation,
            band_start,
            cross_padding,
            (band_end - band_start).max(0.),
            cross,
        );
        let first = if options.reduced_motion && options.kind == CarouselKind::HeroCenter {
            layout_keylines
                .as_ref()
                .map_or(outer, |lines| lines.at(0.).0)
        } else {
            outer
        };
        let mut cursor = first;
        let mut positions = Vec::new();
        let mut anchors = Vec::new();
        let mut max_offset = 0.;
        if valid {
            let gap = if full_screen { FULL_SCREEN_GAP } else { GAP };
            for ratio in aspect_ratios {
                let width = match options.kind {
                    CarouselKind::UncontainedMultiAspectRatio => {
                        let ratio = if ratio.is_finite() {
                            ratio.clamp(9. / 16., 16. / 9.)
                        } else {
                            1.
                        };
                        if horizontal {
                            cross * ratio
                        } else {
                            cross / ratio
                        }
                    }
                    CarouselKind::FullScreen => main,
                    _ => layout_keylines
                        .as_ref()
                        .map_or(preferred, |lines| lines.large),
                }
                .clamp(0., main);
                positions.push((cursor, width));
                anchors.push(cursor - first);
                cursor += width + gap;
            }
            let last_anchor = anchors.last().copied().unwrap_or(0.);
            max_offset = if full_screen
                || options.kind == CarouselKind::HeroCenter
                || (layout_keylines.is_some() && !options.reduced_motion)
            {
                last_anchor
            } else {
                (cursor - gap + band_outer - main).max(0.)
            };
            anchors
                .iter_mut()
                .for_each(|anchor| *anchor = anchor.min(max_offset));
        }
        Self {
            kind: options.kind,
            orientation: options.orientation,
            direction: options.direction,
            viewport,
            main,
            cross: cross_padding,
            clip,
            positions,
            anchors,
            max_offset,
            keylines: layout_keylines.filter(|_| !options.reduced_motion),
        }
    }

    /// Build the geometry directly from caller-provided item descriptors.
    pub fn for_items<M>(options: CarouselOptions<'_>, items: &[CarouselItem<M>]) -> Self {
        let ratios: Vec<_> = items.iter().map(|item| item.aspect_ratio).collect();
        Self::new(options, &ratios)
    }

    pub fn item_count(&self) -> usize {
        self.anchors.len()
    }

    pub fn max_offset(&self) -> f32 {
        self.max_offset
    }

    pub fn item_offset(&self, index: usize) -> Option<f32> {
        self.anchors.get(index).copied()
    }

    pub fn nearest_index(&self, offset: f32) -> Option<usize> {
        let offset = self.clamp(offset);
        let after = self.anchors.partition_point(|anchor| *anchor < offset);
        if after == 0 {
            return self.anchors.first().map(|_| 0);
        }
        self.anchors.get(after).map_or(Some(after - 1), |next| {
            let previous = after - 1;
            Some(if offset - self.anchors[previous] <= *next - offset {
                previous
            } else {
                after
            })
        })
    }

    /// Painted surfaces for the given logical offset. A hidden item has a
    /// zero-area mask. Right-to-left reverses physical positions only.
    pub fn slots(&self, offset: f32) -> Vec<CarouselSlot> {
        let offset = self.clamp(offset);
        if let Some(lines) = &self.keylines {
            let index = offset / (lines.large + GAP);
            let (tail, tail_width) = lines.at(self.item_count().saturating_sub(1) as f32 - index);
            let pad = SIDE_PADDING.min(self.main / 4.);
            let shift = if self.kind == CarouselKind::HeroCenter || index == 0. {
                0.
            } else {
                (self.main - pad - tail - tail_width).max(0.)
            };
            return (0..self.item_count())
                .map(|item| {
                    let relative = item as f32 - index;
                    let (start, width) = lines.at(relative);
                    let hidden = (lines.large - width).max(0.);
                    let art_start =
                        start + shift - hidden / 2. + hidden * 0.25 * relative.clamp(-1., 1.);
                    self.slot(start + shift, width, art_start, lines.large)
                })
                .collect();
        }
        self.positions
            .iter()
            .map(|(start, width)| self.slot(start - offset, *width, start - offset, *width))
            .collect()
    }

    fn slot(&self, start: f32, width: f32, art_start: f32, art_width: f32) -> CarouselSlot {
        let horizontal = self.orientation == CarouselOrientation::Horizontal;
        let mirror = |start, length| {
            if horizontal && self.direction == LayoutDirection::RightToLeft {
                self.main - start - length
            } else {
                start
            }
        };
        let cross = if horizontal {
            self.clip.height
        } else {
            self.clip.width
        };
        let bounds = axis_rect(
            self.orientation,
            mirror(start, width),
            self.cross,
            width,
            cross,
        )
        .intersect(self.clip);
        let artwork = axis_rect(
            self.orientation,
            mirror(art_start, art_width),
            self.cross,
            art_width,
            cross,
        );
        CarouselSlot { bounds, artwork }
    }

    pub(crate) fn clamp(&self, offset: f32) -> f32 {
        if offset.is_finite() {
            offset.clamp(0., self.max_offset)
        } else {
            0.
        }
    }

    pub(crate) fn physical_sign(&self) -> f32 {
        if self.orientation == CarouselOrientation::Horizontal
            && self.direction == LayoutDirection::RightToLeft
        {
            -1.
        } else {
            1.
        }
    }

    pub(crate) fn page_size(&self) -> usize {
        self.keylines.as_ref().map_or_else(
            || {
                self.positions.first().map_or(1, |(_, width)| {
                    (self.main / (width + GAP)).floor().max(1.) as usize
                })
            },
            |lines| lines.large_count,
        )
    }

    pub(crate) fn viewport(&self) -> (f32, f32) {
        self.viewport
    }
}

fn axis_rect(
    orientation: CarouselOrientation,
    main: f32,
    cross: f32,
    length: f32,
    thickness: f32,
) -> Rect {
    match orientation {
        CarouselOrientation::Horizontal => Rect::new(main, cross, length, thickness),
        CarouselOrientation::Vertical => Rect::new(cross, main, thickness, length),
    }
}

fn finite_extent(value: f32) -> f32 {
    if value.is_finite() {
        value.clamp(0., 1_000_000.)
    } else {
        0.
    }
}
